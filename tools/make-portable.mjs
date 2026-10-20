// 从 release 构建产物组装绿色版 zip（免安装；配置存在 exe 同目录）。
// 用法：先 npm run build，再 npm run build:portable
import fs from 'node:fs'
import path from 'node:path'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const root = path.join(fileURLToPath(new URL('..', import.meta.url)))
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'))
const conf = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'))
const productName = conf.productName
const version = conf.version

const exe = path.join(root, 'src-tauri', 'target', 'release', `${pkg.name}.exe`)
const ytdlp = path.join(root, 'src-tauri', 'binaries', 'yt-dlp.exe')
const ffmpeg = path.join(root, 'src-tauri', 'binaries', 'ffmpeg.exe')
for (const f of [exe, ytdlp]) {
  if (!fs.existsSync(f)) {
    throw new Error(`缺少 ${f} —— 请先执行 npm run build`)
  }
}

// 便携目录布局：exe + binaries/（与安装版 resource 布局一致）
const outDir = path.join(root, 'src-tauri', 'target', 'release', 'portable')
const stage = path.join(outDir, productName)
fs.rmSync(stage, { recursive: true, force: true })
fs.mkdirSync(path.join(stage, 'binaries'), { recursive: true })
fs.copyFileSync(exe, path.join(stage, `${productName}.exe`))
fs.copyFileSync(ytdlp, path.join(stage, 'binaries', 'yt-dlp.exe'))
if (fs.existsSync(ffmpeg)) {
  fs.copyFileSync(ffmpeg, path.join(stage, 'binaries', 'ffmpeg.exe'))
}

const zip = path.join(outDir, `${productName}_${version}_Portable.zip`)
fs.rmSync(zip, { force: true })
execFileSync(
  'powershell',
  ['-NoProfile', '-Command', `Compress-Archive -Force -Path '${stage}\\*' -DestinationPath '${zip}'`],
  { stdio: 'inherit' },
)
console.log(`[make-portable] ${zip}`)
