mod douyin_probe;
mod downloader;

use downloader::Downloader;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

type SharedDownloader = Arc<Downloader>;

/// 配置文件路径。绿色版优先：exe 同目录的 config.json（便携模式）；
/// 同目录写不进去时（如安装到 Program Files）退回 app_data_dir。
fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let portable = dir.join("config.json");
            if portable.exists() {
                return Some(portable);
            }
            if std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&portable)
                .is_ok()
            {
                return Some(portable);
            }
        }
    }
    app.path().app_data_dir().ok().map(|d| d.join("config.json"))
}

fn load_download_dir(app: &tauri::AppHandle) -> String {
    if let Some(p) = config_path(app) {
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(dir) = v["downloadDir"].as_str() {
                    if !dir.is_empty() {
                        return dir.to_string();
                    }
                }
            }
        }
    }
    app.path()
        .download_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("C:\\Users\\Public\\Downloads"))
}

fn save_download_dir(app: &tauri::AppHandle, dir: &str) -> Result<(), String> {
    let p = config_path(app).ok_or("无法定位配置目录")?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::json!({ "downloadDir": dir });
    std::fs::write(&p, json.to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_download_dir(state: State<'_, SharedDownloader>) -> String {
    state.download_dir()
}

/// 弹出系统目录选择框；选中后更新并持久化。进行中的任务不受影响（各自持有目录快照）。
#[tauri::command]
fn select_download_dir(
    app: tauri::AppHandle,
    state: State<'_, SharedDownloader>,
) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |file| {
        let _ = tx.send(file.map(|f| f.to_string()));
    });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    if let Some(dir) = &picked {
        state.set_download_dir(dir.clone());
        save_download_dir(&app, dir)?;
    }
    Ok(picked)
}

/// 从粘贴的整段分享文案中提取链接（口令码/话题/标题等杂质自动忽略），逐个入队
#[tauri::command]
fn add_task(state: State<'_, SharedDownloader>, url: String) -> usize {
    downloader::extract_urls(&url)
        .iter()
        .map(|u| state.add_task(u.clone()))
        .count()
}

#[tauri::command]
fn cancel_task(state: State<'_, SharedDownloader>, id: u64) {
    state.cancel_task(id)
}

#[tauri::command]
fn list_tasks(state: State<'_, SharedDownloader>) -> Vec<downloader::Task> {
    state.tasks()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 调试入口：`cargo run -- --probe-douyin <url>` 只跑抖音探测并打印结果
            let args: Vec<String> = std::env::args().collect();
            if let Some(pos) = args.iter().position(|a| a == "--probe-douyin") {
                if let Some(url) = args.get(pos + 1).cloned() {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || {
                        match douyin_probe::probe(&handle, &url) {
                            Ok(o) => println!("PROBE_OK\ttitle={}\turl={}", o.title, o.video_url),
                            Err(e) => println!("PROBE_ERR\t{e}"),
                        }
                        std::process::exit(0);
                    });
                    return Ok(());
                }
            }

            let ytdlp = {
                let bundled = app.path().resource_dir()?.join("binaries").join("yt-dlp.exe");
                if bundled.exists() {
                    bundled
                } else {
                    // 开发环境：resource 目录不存在，直接用仓库里的二进制
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join("yt-dlp.exe")
                }
            };
            if !ytdlp.exists() {
                return Err("未找到 yt-dlp 引擎（src-tauri/binaries/yt-dlp.exe）".into());
            }
            let dir = load_download_dir(app.handle());
            app.manage(Arc::new(Downloader::new(app.handle().clone(), ytdlp, dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_download_dir,
            select_download_dir,
            add_task,
            cancel_task,
            list_tasks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
