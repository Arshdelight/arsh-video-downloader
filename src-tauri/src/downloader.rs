use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

const MAX_CONCURRENT: usize = 3;

pub const EVENT: &str = "tasks-changed";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: u64,
    pub url: String,
    pub title: String,
    /// pending | downloading | done | canceled | error
    pub status: String,
    pub progress: f64,
    pub speed: String,
    /// 任务开始时的目录快照；中途改下载目录不影响进行中的任务
    pub dir: String,
    pub error: String,
}

pub struct Downloader {
    app: AppHandle,
    ytdlp: PathBuf,
    download_dir: Mutex<String>,
    tasks: Mutex<Vec<Task>>,
    children: Mutex<HashMap<u64, Child>>,
    canceled: Mutex<HashSet<u64>>,
    next_id: AtomicU64,
    slots: (Mutex<usize>, Condvar),
}

/// 解析 yt-dlp --newline 的进度行：
/// `[download]  42.3% of  10.53MiB at    3.13MiB/s ETA 00:03`
pub fn parse_progress(line: &str) -> Option<(f64, String)> {
    let rest = line.trim().strip_prefix("[download]")?.trim();
    let percent_part = rest.split_whitespace().next()?;
    let percent: f64 = percent_part.trim_end_matches('%').parse().ok()?;
    let mut speed = String::new();
    if let Some(i) = rest.find(" at ") {
        let tail = &rest[i + 4..];
        let s = tail.split(" ETA").next().unwrap_or("").trim();
        if !s.is_empty() && s != "Unknown" && s != "Unknown speed" {
            speed = s.to_string();
        }
    }
    Some((percent, speed))
}

impl Downloader {
    pub fn new(app: AppHandle, ytdlp: PathBuf, download_dir: String) -> Self {
        Self {
            app,
            ytdlp,
            download_dir: Mutex::new(download_dir),
            tasks: Mutex::new(Vec::new()),
            children: Mutex::new(HashMap::new()),
            canceled: Mutex::new(HashSet::new()),
            next_id: AtomicU64::new(0),
            slots: (Mutex::new(MAX_CONCURRENT), Condvar::new()),
        }
    }

    pub fn download_dir(&self) -> String {
        self.download_dir.lock().unwrap().clone()
    }

    pub fn set_download_dir(&self, dir: String) {
        *self.download_dir.lock().unwrap() = dir;
    }

    pub fn add_task(self: &Arc<Self>, url: String) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        self.tasks.lock().unwrap().push(Task {
            id,
            url: url.clone(),
            title: url,
            status: "pending".into(),
            progress: 0.0,
            speed: String::new(),
            dir: self.download_dir(),
            error: String::new(),
        });
        self.emit_tasks();
        let me = Arc::clone(self);
        thread::spawn(move || me.run(id));
        id
    }

    pub fn cancel_task(&self, id: u64) {
        self.canceled.lock().unwrap().insert(id);
        if let Some(child) = self.children.lock().unwrap().get_mut(&id) {
            let _ = child.kill();
        }
        // 任务可能还在等并发槽位（尚未起进程），补一次状态兜底
        self.update(id, |t| {
            if t.status == "pending" {
                t.status = "canceled".into();
            }
        });
        self.emit_tasks();
    }

    pub fn tasks(&self) -> Vec<Task> {
        self.tasks.lock().unwrap().clone()
    }

    fn run(self: Arc<Self>, id: u64) {
        // 等并发槽位
        let (lock, cvar) = &self.slots;
        let mut available = lock.lock().unwrap();
        while *available == 0 {
            available = cvar.wait(available).unwrap();
        }
        *available -= 1;
        drop(available);

        if self.canceled.lock().unwrap().contains(&id) {
            self.finish(id, "canceled");
            self.release_slot();
            self.emit_tasks();
            return;
        }

        self.update(id, |t| {
            t.status = "downloading".into();
        });
        self.emit_tasks();

        let result = self.spawn_ytdlp(id);

        self.release_slot();
        match result {
            Ok(()) => {
                if self.canceled.lock().unwrap().contains(&id) {
                    self.finish(id, "canceled");
                } else {
                    self.update(id, |t| {
                        t.status = "done".into();
                        t.progress = 100.0;
                        t.speed = String::new();
                    });
                }
            }
            Err(e) => {
                let canceled = self.canceled.lock().unwrap().contains(&id);
                self.update(id, |t| {
                    t.status = if canceled { "canceled".into() } else { "error".into() };
                    if !canceled {
                        t.error = e;
                    }
                });
            }
        }
        self.emit_tasks();
    }

    /// 起进程下载，阻塞到进程退出。标题由 -J 探测，进度走 stdout。
    fn spawn_ytdlp(&self, id: u64) -> Result<(), String> {
        let (url, dir) = {
            let tasks = self.tasks.lock().unwrap();
            let t = tasks.iter().find(|t| t.id == id).ok_or("任务不存在")?;
            (t.url.clone(), t.dir.clone())
        };

        // 先探测标题（-J 只拿元数据，不下视频）
        match self.probe_title(&url) {
            Ok(title) => self.update(id, |t| t.title = title),
            Err(e) => return Err(e),
        }
        if self.canceled.lock().unwrap().contains(&id) {
            return Ok(());
        }

        let mut child = Command::new(&self.ytdlp)
            .args([
                "-f",
                "b",
                "--no-playlist",
                "--newline",
                "--no-warnings",
                "--socket-timeout",
                "30",
                "-P",
                &dir,
                "-o",
                "%(title)s [%(id)s].%(ext)s",
                &url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动 yt-dlp 失败: {e}"))?;

        let stdout = child.stdout.take().ok_or("无法读取 yt-dlp 输出")?;
        let stderr = child.stderr.take();
        self.children.lock().unwrap().insert(id, child);

        // stderr 交给独立线程读净，只留全文供出错时取最后一行
        let last_err = Arc::new(Mutex::new(String::new()));
        if let Some(mut stderr) = stderr {
            let last_err = Arc::clone(&last_err);
            thread::spawn(move || {
                let mut buf = String::new();
                let _ = stderr.read_to_string(&mut buf);
                *last_err.lock().unwrap() = buf;
            });
        }

        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Some((percent, speed)) = parse_progress(&line) {
                self.update(id, |t| {
                    t.progress = percent;
                    t.speed = speed.clone();
                });
                self.emit_tasks();
            }
        }

        let status = {
            let mut children = self.children.lock().unwrap();
            children.remove(&id)
        }
        .ok_or("任务句柄丢失")?
        .wait()
        .map_err(|e| e.to_string())?;

        if status.success() {
            return Ok(());
        }
        let msg = last_err.lock().unwrap().clone();
        Err(if msg.trim().is_empty() {
            format!("yt-dlp 异常退出（{:?}）", status.code())
        } else {
            last_stderr_line(msg.as_bytes())
        })
    }

    fn probe_title(&self, url: &str) -> Result<String, String> {
        let out = Command::new(&self.ytdlp)
            .args(["-J", "--no-playlist", "--no-warnings", "--socket-timeout", "30", url])
            .output()
            .map_err(|e| format!("启动 yt-dlp 失败: {e}"))?;
        if !out.status.success() {
            return Err(last_stderr_line(&out.stderr));
        }
        let v: serde_json::Value =
            serde_json::from_slice(&out.stdout).map_err(|e| format!("解析元数据失败: {e}"))?;
        Ok(v["title"].as_str().unwrap_or(url).to_string())
    }

    fn update(&self, id: u64, f: impl FnOnce(&mut Task)) {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(t) = tasks.iter_mut().find(|t| t.id == id) {
            f(t);
        }
    }

    fn finish(&self, id: u64, status: &str) {
        self.update(id, |t| {
            t.status = status.into();
            t.speed = String::new();
        });
    }

    fn release_slot(&self) {
        *self.slots.0.lock().unwrap() += 1;
        self.slots.1.notify_one();
    }

    fn emit_tasks(&self) {
        let _ = self.app.emit(EVENT, self.tasks());
    }
}

fn last_stderr_line(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("下载失败").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::parse_progress;

    #[test]
    fn parses_percent_and_speed() {
        let (p, s) =
            parse_progress("[download]  42.3% of  10.53MiB at    3.13MiB/s ETA 00:03").unwrap();
        assert!((p - 42.3).abs() < 1e-9);
        assert_eq!(s, "3.13MiB/s");
    }

    #[test]
    fn handles_unknown_speed() {
        let (p, s) =
            parse_progress("[download]   1.0% of ~ 5.00MiB at Unknown speed ETA Unknown").unwrap();
        assert!((p - 1.0).abs() < 1e-9);
        assert_eq!(s, "");
    }

    #[test]
    fn rejects_non_progress_lines() {
        assert!(parse_progress("[download] Destination: C:\\a.mp4").is_none());
        assert!(parse_progress("[generic] Falling back...").is_none());
    }
}
