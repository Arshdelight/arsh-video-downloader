# Arshvideodownloader

Tauri 2 + React + TypeScript desktop app.

## Prerequisites

- Node.js ≥ 20.11
- Rust ≥ 1.77.2（`rustup` — https://rustup.rs）
- WebView2（Windows 10/11 自带）

## Commands

```bash
npm install
npm run dev        # 启动桌面应用（首次会编译 Rust，之后秒开）
npm run dev:web    # 只跑前端（UI 开发，无 Rust 桥接，自动降级）
npm run build      # 类型检查 + 前端构建 + 打包安装器到 src-tauri/target/release/bundle/
npm run lint       # ESLint，零警告
npm test           # vitest
```

## Structure

```
├── src-tauri/           # Rust 后端
│   ├── src/lib.rs       # run() + 所有 #[tauri::command]
│   ├── src/main.rs      # 入口
│   ├── capabilities/    # 权限配置（默认含无边框窗口控制）
│   ├── tauri.conf.json  # 窗口 / 打包 / dev server 配置
│   └── icons/           # 应用图标（npx tauri icon <png> 重新生成）
├── src/
│   ├── ui/              # AppLayout, TitleBar, NavItem, Button, Dialog,
│   │                    # Tooltip, Spinner, Switch + theme tokens (styles.css)
│   ├── lib/             # cn() 和 Tauri 桥接检测
│   ├── App.tsx          # 入口页（含 invoke 示例）
│   └── index.css
├── AGENTS.md            # project conventions for AI coding tools
└── vite.config.ts
```

## Packaging

- 自定义图标：放一张 ≥ 1024×1024 的方形 PNG，跑 `npx tauri icon <png>` 重新生成 `src-tauri/icons/`。
- 安装器元数据在 `src-tauri/tauri.conf.json`（`identifier`、`productName`）。
- 发布体积优化（LTO/strip）在 `src-tauri/Cargo.toml` 的 `[profile.release]`。

## AI coding tools

`AGENTS.md` holds the project conventions. On the first session it walks the
tool through a short interview and writes the final conventions for this
project.
