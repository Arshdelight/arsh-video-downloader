mod douyin_probe;
mod downloader;

use downloader::{Downloader, Task};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

type SharedDownloader = Arc<Downloader>;

/// 数据目录。绿色版优先：exe 同目录（便携模式）；同目录写不进去时
/// （如安装到 Program Files）退回 app_data_dir。
fn data_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let probe = dir.join(".arsh-write-test");
            if std::fs::OpenOptions::new().write(true).create(true).open(&probe).is_ok() {
                let _ = std::fs::remove_file(&probe);
                return Some(dir.to_path_buf());
            }
        }
    }
    app.path().app_data_dir().ok()
}

fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    data_dir(app).map(|d| d.join("config.json"))
}

fn tasks_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    data_dir(app).map(|d| d.join("tasks.json"))
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
    let p = config_path(app).ok_or("无法定位数据目录")?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::json!({ "downloadDir": dir });
    std::fs::write(&p, json.to_string()).map_err(|e| e.to_string())
}

/// 恢复历史任务（跨会话持久化）。上次退出时还在排队/下载中的任务由
/// Downloader::new 统一标记为已暂停。
fn load_tasks(app: &tauri::AppHandle) -> Vec<Task> {
    let Some(p) = tasks_path(app) else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(&p) else { return Vec::new() };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| serde_json::from_value::<Vec<Task>>(v["tasks"].clone()).ok())
        .unwrap_or_default()
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
fn pause_task(state: State<'_, SharedDownloader>, id: u64) {
    state.pause_task(id)
}

#[tauri::command]
fn start_task(state: State<'_, SharedDownloader>, id: u64) {
    state.start_task(id)
}

#[tauri::command]
fn delete_task(state: State<'_, SharedDownloader>, id: u64) {
    state.delete_task(id)
}

#[tauri::command]
fn clear_finished(state: State<'_, SharedDownloader>) -> usize {
    state.clear_finished()
}

#[tauri::command]
fn list_tasks(state: State<'_, SharedDownloader>) -> Vec<Task> {
    state.tasks()
}

/// 打开任务所在目录；已完成的文件在资源管理器中定位选中
#[tauri::command]
fn open_task_dir(
    app: tauri::AppHandle,
    state: State<'_, SharedDownloader>,
    id: u64,
) -> Result<(), String> {
    let task = state.task(id).ok_or("任务不存在")?;
    let opener = app.opener();
    if let Some(fp) = &task.file_path {
        if std::path::Path::new(fp).exists() {
            return opener.reveal_item_in_dir(fp).map_err(|e| e.to_string());
        }
    }
    std::fs::create_dir_all(&task.dir).map_err(|e| e.to_string())?;
    opener.open_path(&task.dir, None::<&str>).map_err(|e| e.to_string())
}

/// 用系统默认播放器打开下载完成的文件
#[tauri::command]
fn open_task_file(
    app: tauri::AppHandle,
    state: State<'_, SharedDownloader>,
    id: u64,
) -> Result<(), String> {
    let task = state.task(id).ok_or("任务不存在")?;
    let path = task.file_path.ok_or("文件尚未下载完成")?;
    if !std::path::Path::new(&path).exists() {
        return Err("文件不存在，可能已被移动或删除".into());
    }
    app.opener().open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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
            let tasks = load_tasks(app.handle());
            let persist = tasks_path(app.handle());
            app.manage(Arc::new(Downloader::new(
                app.handle().clone(),
                ytdlp,
                dir,
                persist,
                tasks,
            )));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_download_dir,
            select_download_dir,
            add_task,
            cancel_task,
            pause_task,
            start_task,
            delete_task,
            clear_finished,
            list_tasks,
            open_task_dir,
            open_task_file
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 退出时把任务列表落盘，下次启动恢复
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<SharedDownloader>() {
                    state.persist();
                }
            }
        });
}
