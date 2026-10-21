//! 解析 sidecar：调用打包好的 ParseHub（Python 库）可执行文件，
//! 抖音等平台的反爬签名由上游库（MIT, https://github.com/z-mio/ParseHub）维护，
//! 本应用不自行实现签名算法。

use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const PROBE_TIMEOUT: Duration = Duration::from_secs(120);

pub struct ProbeOutcome {
    pub title: String,
    /// ParseHub 清洗过的文件安全名（可直接用作下载文件名）
    pub name: String,
    pub video_url: String,
}

#[derive(Deserialize)]
struct Report {
    ok: bool,
    #[serde(default)]
    title: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    video_url: String,
    #[serde(default)]
    error: String,
}

fn sidecar_exe(app: &AppHandle) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("binaries")
        .join("parsehub-sidecar")
        .join("parsehub-probe.exe");
    if bundled.exists() {
        return Ok(bundled);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("parsehub-sidecar")
        .join("parsehub-probe.exe");
    if dev.exists() {
        return Ok(dev);
    }
    Err("未找到解析引擎（binaries/parsehub-sidecar），请重新安装或查看 README".into())
}

fn spawn_sidecar(exe: &PathBuf, url: &str) -> Result<std::process::Child, String> {
    let mut cmd = Command::new(exe);
    cmd.arg(url).stdout(Stdio::piped()).stderr(Stdio::piped());
    // sidecar 是 PyInstaller 打包的 Python 程序。注：PyInstaller bootloader
    // 会忽略 PYTHONUTF8 等变量，UTF-8 输出由 parsehub_probe.py 内部
    // reconfigure 保证，这里注入仅作语义说明
    cmd.env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().map_err(|e| format!("启动解析引擎失败: {e}"))
}

/// 解析链接，返回标题/文件名/直链。阻塞至 sidecar 退出或超时。
pub fn probe(app: &AppHandle, url: &str) -> Result<ProbeOutcome, String> {
    let exe = sidecar_exe(app)?;
    let mut child = spawn_sidecar(&exe, url)?;

    let mut stdout = child.stdout.take().ok_or("无法读取解析引擎输出")?;
    let stderr = child.stderr.take();

    // stdout 在独立线程读到 EOF（按字节容错编码），主线程带超时等待；超时杀进程兜底
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    if let Some(mut stderr) = stderr {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf);
        });
    }

    let output = match rx.recv_timeout(PROBE_TIMEOUT) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => {
            let _ = child.kill();
            return Err("解析超时，请检查网络后重试".into());
        }
    };
    let status = child.wait().map_err(|e| e.to_string())?;
    let _ = status;

    let line = output
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| "解析引擎无有效输出".to_string())?;
    let report: Report = serde_json::from_str(line.trim())
        .map_err(|e| format!("解析引擎输出异常: {e}"))?;

    if report.ok && !report.video_url.is_empty() {
        Ok(ProbeOutcome {
            title: if report.title.is_empty() { url.into() } else { report.title },
            name: report.name,
            video_url: report.video_url,
        })
    } else {
        Err(if report.error.is_empty() { "解析失败".into() } else { report.error })
    }
}
