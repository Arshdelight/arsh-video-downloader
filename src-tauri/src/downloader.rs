use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows 下隐藏子进程的控制台窗口（GUI 程序拉起 yt-dlp 时会弹黑框）
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn new_command(ytdlp: &PathBuf) -> Command {
    let mut cmd = Command::new(ytdlp);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// 从整段分享文案里提取链接：只有 http(s)/www 开头的 token 才算，
/// 口令码、话题、标题等杂质直接丢弃，并去掉粘在链接尾部的中文标点。
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for token in text.split_whitespace() {
        let is_url =
            token.starts_with("http://") || token.starts_with("https://") || token.starts_with("www.");
        if !is_url {
            continue;
        }
        let cleaned = token
            .trim_end_matches([
                '，', '。', '！', '？', '；', '、', '）', '】', '》', '"', '"', '\u{2018}',
                '\u{2019}', '\'', '"',
            ])
            .to_string();
        if !cleaned.is_empty() && !urls.contains(&cleaned) {
            urls.push(cleaned);
        }
    }
    urls
}

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

        // 抖音：yt-dlp 的提取器无法过抖音风控（detail API 强制 a_bogus 签名，cookie 无关），
        // 改用隐藏 WebView 在页面上下文里拿签名过的直链，yt-dlp 只负责纯下载
        let is_douyin = url.contains("douyin.com");
        let (title, target, output_template) = if is_douyin {
            let outcome = crate::douyin_probe::probe(&self.app, &url)?;
            let template = format!("{}.%(ext)s", sanitize_filename(&outcome.title));
            (outcome.title, outcome.video_url, template)
        } else {
            // 先探测标题（-J 只拿元数据，不下视频）
            let title = self.probe_title(&url)?;
            (title, url, "%(title)s [%(id)s].%(ext)s".to_string())
        };
        self.update(id, |t| t.title = title);
        if self.canceled.lock().unwrap().contains(&id) {
            return Ok(());
        }

        let mut child = new_command(&self.ytdlp)
            .args([
                "-f",
                "b",
                "--no-playlist",
                "--newline",
                "--no-warnings",
                "--socket-timeout",
                "30",
                // Errno 22 两大来源的兜底：不回写文件 mtime；强制 Windows 安全文件名
                "--no-mtime",
                "--windows-filenames",
                "-P",
                &dir,
                "-o",
                &output_template,
                &target,
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
            last_stderr_context(msg.as_bytes())
        })
    }

    fn probe_title(&self, url: &str) -> Result<String, String> {
        let out = new_command(&self.ytdlp)
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

/// 下载失败时保留 stderr 末尾几行作为任务错误信息，便于定位
fn last_stderr_context(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = lines.len().saturating_sub(4);
    let joined = lines[start..].join(" ｜ ");
    let mut chars: Vec<char> = joined.chars().collect();
    if chars.len() > 400 {
        chars = chars.split_off(chars.len() - 400);
    }
    if chars.is_empty() { "下载失败".into() } else { chars.into_iter().collect() }
}

/// 文件名安全化：去掉控制字符/零宽字符（抖音简介里常见，Windows 打开会 EINVAL）、
/// 替换文件系统非法字符、截断长度
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
        })
        .map(|c| {
            if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']).to_string();
    let short: String = trimmed.chars().take(80).collect();
    if short.is_empty() { "video".into() } else { short }
}

#[cfg(test)]
mod tests {
    use super::{extract_urls, parse_progress, sanitize_filename};

    #[test]
    fn extracts_urls_from_share_text() {
        let text = "8.71 :7pm 01/14 CHi:/ D@u.FU 全网创一代择偶真心话，就听我这条就够了，笑死 # 情感 # 创一代  https://v.douyin.com/WkrVLq3vOFY/ 复制此链接，打开Dou音搜索，直接观看视频！";
        assert_eq!(extract_urls(text), vec!["https://v.douyin.com/WkrVLq3vOFY/"]);
    }

    #[test]
    fn trims_trailing_chinese_punctuation() {
        assert_eq!(
            extract_urls("好看 https://v.douyin.com/abc123，"),
            vec!["https://v.douyin.com/abc123"]
        );
    }

    #[test]
    fn dedupes_and_ignores_non_urls() {
        assert_eq!(extract_urls("https://a.com https://a.com # 情感 8.71"), vec!["https://a.com"]);
        assert!(extract_urls("没有链接 # 话题 8.71 :7pm").is_empty());
    }

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

    #[test]
    fn sanitizes_invisible_and_illegal_chars() {
        // 零宽字符 / 双向控制符（抖音简介常见）会被 Windows open() 以 EINVAL 拒绝
        assert_eq!(
            sanitize_filename("当羊羊\u{200b}撞车\u{202e}S1: 对局*好看?"),
            "当羊羊撞车S1_ 对局_好看_"
        );
        assert_eq!(sanitize_filename("  尾巴点.  "), "尾巴点");
        assert_eq!(sanitize_filename("\u{200b}\u{feff}"), "video");
    }
}
