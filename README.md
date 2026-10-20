# Arsh Video Downloader

[![Release](https://img.shields.io/github/v/release/Arshdelight/arsh-video-downloader)](https://github.com/Arshdelight/arsh-video-downloader/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

轻量通用视频下载器：粘贴链接或整段分享文案，自动识别并下载。Tauri 2 + React + TypeScript，安装包仅数 MB。

> A lightweight universal video downloader (Douyin / Bilibili / YouTube / Twitter ...) built with Tauri 2. GUI wrapper around the `ParseHub` + `yt-dlp` + `ffmpeg` stack.

## ✨ 功能

- **粘贴即下载** — 支持整段分享文案（自动提取其中的链接，口令/话题等杂质自动忽略）
- **下载队列** — 最多 3 并发，进度条、实时速度、暂停/继续（断点续传）、单任务删除
- **抖音免登录** — 隐藏 WebView 页内签名方案：让抖音自己的前端 JS 给请求签名，无需登录、无需逆向、不打包浏览器
- **B 站高清** — 内置 ffmpeg，DASH 分离流自动合并，未登录最高 1080P
- **打开/定位** — 已完成文件一键打开或资源管理器定位
- **跨会话持久化** — 任务列表重启恢复

## ⚙️ 工作原理

```
粘贴文案 ──► 链接提取 ──► ┌─ 抖音 ────► ParseHub sidecar（页内反爬签名由上游库维护）
                          └─ 其他平台 ─► yt-dlp -J 探测
                                      │
                                      ▼
                        yt-dlp 纯下载（直链）+ ffmpeg 合并 DASH
```

| 引擎 | 职责 | 说明 |
|---|---|---|
| [ParseHub](https://github.com/z-mio/ParseHub) (MIT) | 抖音等国内平台解析 | 反爬签名随上游更新，本应用不自研签名算法 |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) (Unlicense) | 其他平台解析 + 全平台下载 | 断点续传、进度解析 |
| ffmpeg | 合并音视频分离流（DASH） | 随应用分发 |

## 📦 下载

到 [Releases](https://github.com/Arshdelight/arsh-video-downloader/releases) 获取：

- `ArshVideoDownloader_x.y.z_x64-setup.exe` — 安装版（支持应用内自动更新）

## 🛠 开发

前置要求：Node.js ≥ 20.11、Rust ≥ 1.77.2、WebView2（Win10/11 自带）。

```bash
npm install
npm run dev        # 启动桌面应用（首次会编译 Rust，之后秒开）
npm run dev:web    # 只跑前端（无 Rust 桥接，自动降级）
npm run lint && npm test
```

### 引擎二进制（构建前下载，均已 gitignore）

```bash
# yt-dlp（必需）
curl -L -o src-tauri/binaries/yt-dlp.exe https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe

# ffmpeg（必需，合并 DASH 分离流；winget 安装后复制，或任一下载渠道）
winget install Gyan.FFmpeg
# 复制 %LOCALAPPDATA%\Microsoft\WinGet\Packages\Gyan.FFmpeg*\*\bin\ffmpeg.exe 到 src-tauri/binaries/

# ParseHub 解析 sidecar（会自动建 venv 并用 PyInstaller 打包，详见 sidecar/build.ps1）
powershell -NoProfile -ExecutionPolicy Bypass -File sidecar/build.ps1
```

### 打包

```bash
npm run build            # NSIS 安装器 → src-tauri/target/release/bundle/nsis/
```

- 自定义图标：源文件为 `app-icon.svg`，修改后栅格化为 1024×1024 PNG（如用 headless Edge 截图），再跑
  `npx tauri icon app-icon.png` 重新生成 `src-tauri/icons/`（含安装包使用的 icon.ico）。

### 发布与自动更新

安装版内置应用内自动更新（启动时静默检查 → 一键更新），基于
[tauri-plugin-updater](https://v2.tauri.app/plugin/updater/) + GitHub Releases：

1. 生成一次签名密钥（私钥妥善保管，不入库）：
   `npx tauri signer generate -w ~/.tauri/arsh-video-downloader.key`
2. 仓库 Secrets 配置 `TAURI_SIGNING_PRIVATE_KEY`（私钥内容）与
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（空密码留空）
3. 推送 `v*` tag → GitHub Actions 自动构建、签名、发布安装包和 `latest.json`

本地打包安装版同样需要签名密钥：设置环境变量 `TAURI_SIGNING_PRIVATE_KEY`
（私钥文件内容）。

## 🗂 Structure

```
├── src-tauri/
│   ├── src/lib.rs           # 命令层：目录选择/持久化、任务增删改查、退出落盘
│   ├── src/downloader.rs    # 下载管理器：队列并发、yt-dlp 进程、进度/暂停/续传
│   ├── src/parsehub_probe.rs# 解析 sidecar 调用（抖音等国内平台）
│   ├── binaries/            # yt-dlp.exe / ffmpeg.exe / parsehub-sidecar（不入库）
│   └── tauri.conf.json
├── sidecar/                 # ParseHub sidecar 入口 + PyInstaller 打包脚本
├── src/
│   ├── ui/                  # ArshUI 组件 + 主题 tokens
│   ├── lib/                 # api 封装、Tauri 桥接检测
│   └── App.tsx              # 粘贴框 + 目录行 + 下载队列（右键菜单/快捷操作）
└── vite.config.ts
```

## 📄 License

[MIT](LICENSE)。 yt-dlp / ParseHub / ffmpeg 为独立项目，遵循其各自许可证；本应用以独立进程调用它们。
