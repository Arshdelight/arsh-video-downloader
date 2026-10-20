mod downloader;
mod parsehub_probe;

use downloader::{Downloader, Task};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

type SharedDownloader = Arc<Downloader>;

/// 当前待安装的更新（check_update 后暂存，install_update 消费）
struct UpdateState(std::sync::Mutex<Option<tauri_plugin_updater::Update>>);

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

/// 当前 exe 是否为安装版（绿色版走"前往下载页"更新，安装版走应用内更新）
#[tauri::command]
fn is_installed() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("uninstall.exe").exists()))
        .unwrap_or(false)
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

/// 引擎就绪状态（设置页展示）；与 setup 的解析逻辑保持一致
#[tauri::command]
fn engine_status(app: tauri::AppHandle) -> serde_json::Value {
    let dev_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
    let bundled = |name: &str| -> bool {
        app.path()
            .resource_dir()
            .map(|d| d.join("binaries").join(name).exists())
            .unwrap_or(false)
            || dev_bin.join(name).exists()
    };
    let sidecar = {
        let bundled = app
            .path()
            .resource_dir()
            .map(|d| {
                d.join("binaries")
                    .join("parsehub-sidecar")
                    .join("parsehub-probe.exe")
                    .exists()
            })
            .unwrap_or(false)
            || dev_bin
                .join("parsehub-sidecar")
                .join("parsehub-probe.exe")
                .exists();
        bundled
    };
    serde_json::json!({ "ytdlp": bundled("yt-dlp.exe"), "ffmpeg": bundled("ffmpeg.exe"), "parsehub": sidecar })
}

/// 打开数据目录（config.json / tasks.json 所在位置）
#[tauri::command]
fn open_data_dir(app: tauri::AppHandle) -> Result<(), String> {
    let dir = data_dir(&app).ok_or("无法定位数据目录")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_repo(app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_url("https://github.com/Arshdelight/arsh-video-downloader", None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_update(
    app: tauri::AppHandle,
    state: State<'_, UpdateState>,
) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    let version = update.as_ref().map(|u| u.version.clone());
    *state.0.lock().unwrap() = update;
    Ok(version)
}

/// 下载并静默安装更新，完成后自动重启应用（仅安装版）
#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    state: State<'_, UpdateState>,
) -> Result<(), String> {
    let update = state
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or("没有待安装的更新，请先检查更新")?;
    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
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
    // 已完成的任务：尝试定位到具体文件
    if task.status == "done" {
        if let Ok((_, path)) = state.resolve_file_path(id) {
            return opener.reveal_item_in_dir(&path).map_err(|e| e.to_string());
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
    let (_, path) = state.resolve_file_path(id)?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(UpdateState(std::sync::Mutex::new(None)));
            // 调试入口：`cargo run -- --probe <url>` 只跑解析 sidecar 并打印结果
            let args: Vec<String> = std::env::args().collect();
            if let Some(pos) = args.iter().position(|a| a == "--probe") {
                if let Some(url) = args.get(pos + 1).cloned() {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || {
                        match parsehub_probe::probe(&handle, &url) {
                            Ok(o) => println!("PROBE_OK\ttitle={}\tname={}\turl={}", o.title, o.name, o.video_url),
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
            // ffmpeg 可选：缺失时音视频分离的流（B站高清等）无法合并，会退回单文件格式
            let ffmpeg = {
                let bundled = app.path().resource_dir()?.join("binaries").join("ffmpeg.exe");
                if bundled.exists() {
                    Some(bundled)
                } else {
                    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join("ffmpeg.exe");
                    dev.exists().then_some(dev)
                }
            };
            let dir = load_download_dir(app.handle());
            let tasks = load_tasks(app.handle());
            let persist = tasks_path(app.handle());
            app.manage(Arc::new(Downloader::new(
                app.handle().clone(),
                ytdlp,
                ffmpeg,
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
            open_task_file,
            is_installed,
            check_update,
            install_update,
            get_app_version,
            engine_status,
            open_data_dir,
            open_repo
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
