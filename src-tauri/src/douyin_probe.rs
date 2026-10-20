//! 抖音探测：yt-dlp 的 Douyin 提取器已无法通过抖音风控（detail API 强制 a_bogus
//! 签名，与 cookie 无关）。改用"页面上下文签名"方案：创建隐藏 WebView 加载视频页，
//! 抖音自己的前端 JS 会给页面内发出的 fetch 自动附加签名，拿到 detail JSON 后
//! 通过本地回环 HTTP 回传给 Rust，从中提取直链和标题交给下载器。

use serde::Deserialize;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

const PROBE_TIMEOUT: Duration = Duration::from_secs(75);

#[derive(Deserialize)]
struct Report {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    title: String,
    #[serde(default)]
    video_url: String,
    #[serde(default)]
    error: String,
}

pub struct ProbeOutcome {
    pub title: String,
    pub video_url: String,
}

fn initialization_script(port: u16) -> String {
    format!(
        r#"
(async () => {{
  const report = (obj) => {{
    try {{
      fetch('http://127.0.0.1:{port}/report', {{
        method: 'POST',
        mode: 'no-cors',
        headers: {{ 'Content-Type': 'text/plain' }},
        body: JSON.stringify(obj),
      }});
    }} catch (e) {{}}
  }};
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  try {{
    // 短链会先落在 v.douyin.com 再重定向到 /video/ 或 /note/ 页，
    // 等定位到视频页且页面自身的签名钩子就绪后再发请求
    for (let i = 0; i < 45; i++) {{
      await wait(1000);
      const m = location.pathname.match(/\/(?:video|note)\/(\d+)/);
      if (!m) continue;
      const id = m[1];
      const api = 'https://www.douyin.com/aweme/v1/web/aweme/detail/?device_platform=webapp&aid=6383&channel=channel_pc_web&aweme_id=' + id;
      try {{
        const resp = await fetch(api, {{ credentials: 'include' }});
        const text = await resp.text();
        let data = null;
        try {{ data = JSON.parse(text); }} catch {{}}
        const d = data && data.aweme_detail;
        if (d) {{
          const play = d.video && (d.video.play_addr || d.video.download_addr);
          const url = play && play.url_list && play.url_list[0];
          if (url) {{
            report({{ ok: true, title: d.desc || id, video_url: url }});
            return;
          }}
        }}
      }} catch (e) {{ /* 本轮失败，下轮重试 */ }}
    }}
    report({{ ok: false, error: '页面加载超时或未取到视频数据' }});
  }} catch (e) {{
    report({{ ok: false, error: String(e) }});
  }}
}})();
"#
    )
}

fn read_report(stream: &mut std::net::TcpStream) -> Option<Report> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = b"\r\n\r\n";
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_subslice(&buf, header_end) {
                    let headers = String::from_utf8_lossy(&buf[..pos]).to_lowercase();
                    let content_length: usize = headers
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(0);
                    let body_start = pos + header_end.len();
                    let target = body_start + content_length;
                    while buf.len() < target {
                        match stream.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                            Err(_) => break,
                        }
                    }
                    let end = target.min(buf.len());
                    return serde_json::from_slice(&buf[body_start..end]).ok();
                }
            }
            Err(_) => break,
        }
    }
    None
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// 打开隐藏 WebView 加载视频页（支持短链/长链），等待页内脚本回传直链。
pub fn probe(app: &AppHandle, page_url: &str) -> Result<ProbeOutcome, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("绑定本地端口失败: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;

    let label = format!("douyin-probe-{port}");
    let window = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(page_url.parse().map_err(|e| format!("解析页面地址失败: {e}"))?),
    )
    .title("douyin-probe")
    .visible(false)
    .inner_size(420.0, 320.0)
    .initialization_script(&initialization_script(port))
    .build()
    .map_err(|e| format!("创建探测窗口失败: {e}"))?;

    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut outcome: Result<ProbeOutcome, String> =
        Err("探测超时：抖音页面未能在限时内回传数据".into());

    loop {
        if Instant::now() >= deadline {
            break;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                // 无论是否解析成功，都回一个 HTTP 200 让页面端 fetch 结束
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                );
                if let Some(report) = read_report(&mut stream) {
                    if report.ok && !report.video_url.is_empty() {
                        outcome = Ok(ProbeOutcome {
                            title: if report.title.is_empty() { "douyin".into() } else { report.title },
                            video_url: report.video_url,
                        });
                    } else {
                        outcome = Err(if report.error.is_empty() {
                            "探测失败：未取到视频直链".into()
                        } else {
                            report.error
                        });
                    }
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(150));
            }
            Err(e) => {
                outcome = Err(format!("本地回环接收失败: {e}"));
                break;
            }
        }
    }

    let _ = window.close();
    outcome
}

#[cfg(test)]
mod tests {
    use super::initialization_script;

    #[test]
    fn script_embeds_port_and_api() {
        let s = initialization_script(45678);
        assert!(s.contains("127.0.0.1:45678/report"));
        assert!(s.contains("aweme/v1/web/aweme/detail/"));
        assert!(s.contains("/(?:video|note)"));
    }
}
