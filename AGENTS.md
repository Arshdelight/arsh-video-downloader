# Arsh Video Downloader

轻量通用视频下载器（抖音 / B站 / YouTube / Twitter 等）：粘贴链接或整段分享文案，自动解析并下载。技术栈为 Tauri 2（Rust）+ React 19 + TypeScript + Tailwind 4；解析层由 ParseHub sidecar（抖音等国内平台，反爬签名随上游维护）与 yt-dlp（其他平台）承担，下载与 DASH 合并由 yt-dlp + ffmpeg 完成。发布形态为 NSIS 安装版（含应用内自动更新）。

## 编码原则

- **先想后写**：动手前理清需求；有假设明说，有取舍摆出来，需求不合理就直说
- **极简实现**：写能解决问题的最少代码，不做投机性设计
- **外科手术式改动**：只改必须改的地方，只清理自己制造的混乱，不顺手重构
- **目标驱动**：动手前先定成功标准，完成后按标准验证，未通过就继续修
- 琐碎改动（错别字、显而易见的一行修复）不必走以上流程

## Commit 规范

在本文件存在期间，可以频繁提交 commit。格式：**英文前缀 + 中文描述**，一行不超过 72 字符。

```
<prefix>: <中文描述>
```

可用前缀：

- `feat`: 新功能
- `fix`: 修复 bug
- `docs`: 文档变更
- `style`: 代码格式（不影响逻辑）
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建、依赖、工具等杂项
- `build`: 构建产物、打包配置相关

示例：

```
feat: 抖音下载走隐藏 WebView 页内签名，绕过 a_bogus 风控
fix: 修复管道 UTF-8 断流导致的下载 EINVAL (22)
build: 只打包 chrome-headless-shell，登录改用系统浏览器
```

**红线：不自动 push，一切 push / 推 `v*` tag 由用户指令驱动。推 `v*` tag 会触发 GitHub Actions 正式发布（构建、签名、发 Release），日常开发不要随手推 tag。**

## 协作方式

- 日常修 bug、加功能直接动手；引擎选型、开源策略、发布节奏这类大决策先给方案征得同意
- 需求以问号结尾的提问只要求回答，不据此动手改代码

## 开发注意事项

- **引擎二进制不入库**：`src-tauri/binaries/` 下的 yt-dlp.exe、ffmpeg.exe、parsehub-sidecar/ 均被 gitignore，构建前按 README 下载/生成（sidecar 用 `sidecar/build.ps1` 打包）
- **本地打安装包需要签名**：设置 `TAURI_SIGNING_PRIVATE_KEY`（私钥为本仓库签名密钥，生成时输出到 `~/.tauri/`，丢失 = 已发布用户的更新链路断裂；GitHub Secrets 里有一份供 CI 使用）
- **子进程管道必须按字节读 + lossy 解码**：yt-dlp/sidecar 输出可能含非 UTF-8 字节，`lines()`/`read_to_string` 会断流导致下载 EINVAL (22)；Python 子进程一律注入 `PYTHONUTF8=1`
- **落盘文件名必须过 `sanitize_filename`**：抖音简介常含控制/零宽字符，Windows 会以 EINVAL 拒绝
- 图标源文件为 `app-icon.svg`，改动后栅格化再 `npx tauri icon` 重新生成全套

## 测试规范

- 交付前四件套必须全绿：`npm run lint`（零警告）+ `npx tsc --noEmit` + `npm test` + `src-tauri` 下 `cargo test`
- 改动 Rust 代码或 capabilities 时加跑 `cargo check`
- 改动解析/下载引擎时，用真实链接实测：抖音口令短链 + B站 BV 链接各一条（调试入口 `cargo run -- --probe <url>`）
- 动了构建/打包配置时，装一次新安装器冒烟（能装、能启动、图标正常）
- 不碰的东西：用户下载目录里的真实文件；GitHub 上已发布的 Release（发坏了用新版本覆盖，不删 release）

## 沟通规范

- **提问不是任务指令**：用户以问号结尾的提问（"可以吗""是什么""怎么做""有什么区别"）只要求回答，不要据此开始改代码或动手实现；先给出结论和说明，等用户明确说"改/做/加"后再动手
- 对拿不准是否要动手的情况，先说明打算做什么，征得同意再改

## 任务汇报规范

完成任务后，按以下格式向用户汇报：

```
本轮任务目标：xxx
具体完成内容：
1. xxx（相关文件：xxx）
2. xxx（相关文件：xxx）
3. xxx（相关文件：xxx）
下一步建议：xxx
```
