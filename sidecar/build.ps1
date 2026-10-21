# 打包 ParseHub 解析 sidecar 为 onedir exe（输出 src-tauri/binaries/parsehub-sidecar/）
# 用法: powershell -NoProfile -ExecutionPolicy Bypass -File sidecar/build.ps1
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$sidecarDir = Join-Path $root 'sidecar'
$venvDir = Join-Path $sidecarDir '.venv'
$outDir = Join-Path $root 'src-tauri' | Join-Path -ChildPath 'binaries' | Join-Path -ChildPath 'parsehub-sidecar'

if (-not (Test-Path (Join-Path $venvDir 'Scripts/python.exe'))) {
    python -m venv $venvDir
}
$py = Join-Path $venvDir 'Scripts/python.exe'
& $py -m pip install -q --upgrade pip
# 只装库本体（[cli] 扩展会带入大量 CLI 专用依赖），sidecar 只用库 API。
# parsehub 锁定已验证版本：不锁的话发版可能撞上游新版本的打包回归
# （v0.1.5 就因此打进了 150MB 的 cv2）；升级时改版本号并靠 CI 守卫验证
& $py -m pip install -q 'parsehub==2.3.13' 'pyinstaller'

$workDir = Join-Path $sidecarDir 'build-work'
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
Push-Location $workDir
# 排除 cv2/numpy/PIL（约 150MB）：仅 parsehub.utils.media_info 用于读取
# 微博/Twitter 图集落盘后的文件信息，探针不执行该路径；运行时由
# parsehub_probe.py 注入空壳模块顶住 eager import 链
& $py -m PyInstaller --noconfirm --clean --onedir --name parsehub-probe `
    --hidden-import parsehub --collect-submodules parsehub `
    --collect-data urlextract `
    --exclude-module cv2 --exclude-module numpy --exclude-module PIL `
    (Join-Path $sidecarDir 'parsehub_probe.py')
Pop-Location

if (Test-Path $outDir) {
    Remove-Item -Recurse -Force $outDir
}
Move-Item (Join-Path $workDir 'dist/parsehub-probe') $outDir
Write-Host "[build-sidecar] $outDir"
