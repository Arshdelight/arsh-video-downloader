# Arsh Video Downloader

轻量通用视频下载器：粘贴任意平台的视频链接，走 yt-dlp 引擎解析直链并并发下载。Tauri 2 + React + TypeScript，安装包只有几 MB。

## 功能

- 粘贴链接即下载，支持一次粘贴多个（空格分隔）；解析由内置 yt-dlp 完成（抖音、B站、YouTube、Twitter 等数千个站点）
- 下载队列：最多 3 个并发，带进度条、实时速度、单任务取消
- 下载目录可自由选择，持久化保存；**中途更换目录不影响进行中的任务**（每个任务持有开始时的目录快照）
- 无边框窗口 + 自定义标题栏（ArshUI 风格）

## 开发

```bash
npm install
npm run dev        # 启动桌面应用（首次会编译 Rust，之后秒开）
npm run dev:web    # 只跑前端（UI 开发，无 Rust 桥接，自动降级）
npm run build      # 类型检查 + 前端构建 + 打包安装器到 src-tauri/target/release/bundle/
npm run lint       # ESLint，零警告
npm test           # vitest
cargo test         # Rust 侧测试（进度行解析），在 src-tauri/ 下执行
```

前置要求：Node.js ≥ 20.11、Rust ≥ 1.77.2、WebView2（Win10/11 自带）。

构建前下载引擎二进制（已 gitignore，不入库）：

```bash
curl -L -o src-tauri/binaries/yt-dlp.exe https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe
```

## Structure

```
├── src-tauri/
│   ├── src/lib.rs       # 命令：目录选择/持久化、任务的增查删
│   ├── src/downloader.rs# 下载管理器：队列并发、yt-dlp 进程、进度解析、取消
│   ├── binaries/yt-dlp.exe  # 下载引擎（随应用打包）
│   ├── capabilities/    # 权限：窗口控制 + 目录选择对话框
│   └── tauri.conf.json
├── src/
│   ├── ui/              # ArshUI 组件 + 主题 tokens
│   ├── lib/             # api 封装、Tauri 桥接检测、事件名
│   └── App.tsx          # 粘贴框 + 目录行 + 下载队列
└── vite.config.ts
```

## Notes

- 视频直链的获取完全交给 yt-dlp（签名、反爬由其社区维护），升级引擎只需替换 `src-tauri/binaries/yt-dlp.exe`
- 未打包 ffmpeg：音视频分离的流（如 B 站高清晰度）会自动退回单流格式；需要最高画质时后续再考虑内置 ffmpeg
- 下载目录持久化在 `%APPDATA%/com.arshdelight.arshvideodownloader/config.json`

## AI coding tools

`AGENTS.md` holds the project conventions. On the first session it walks the
tool through a short interview and writes the final conventions for this
project.
