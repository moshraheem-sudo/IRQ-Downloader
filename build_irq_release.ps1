$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat'
if (-not (Test-Path $vcvars)) { throw "Visual Studio C++ Build Tools were not found: $vcvars" }
Push-Location $root
try {
    Write-Host 'Building IRQ Downloader (Rust release)...' -ForegroundColor Cyan
    & $env:ComSpec /d /s /c "call `"$vcvars`" -arch=x64 -host_arch=x64 >nul && cargo build --workspace --release"
    if ($LASTEXITCODE -ne 0) { throw "Cargo build failed with exit code $LASTEXITCODE" }

    $exe = Join-Path $root 'target\release\IRQDownloader.exe'
    if (-not (Test-Path $exe)) { throw "Release executable not found: $exe" }
    $dist = Join-Path $root 'dist\IRQ Downloader'
    New-Item -ItemType Directory -Force -Path $dist | Out-Null
    Copy-Item $exe (Join-Path $dist 'IRQDownloader.exe') -Force
    New-Item -ItemType Directory -Force -Path (Join-Path $dist 'tools'), (Join-Path $dist 'extension'), (Join-Path $dist 'extension-firefox') | Out-Null
    Copy-Item (Join-Path $root 'tools\*') (Join-Path $dist 'tools') -Recurse -Force
    Copy-Item (Join-Path $root 'extension\*') (Join-Path $dist 'extension') -Recurse -Force
    Copy-Item (Join-Path $root 'extension\*') (Join-Path $dist 'extension-firefox') -Recurse -Force
    Copy-Item (Join-Path $root 'extension\manifest.firefox.json') (Join-Path $dist 'extension-firefox\manifest.json') -Force
    Copy-Item (Join-Path $root 'LICENSE') (Join-Path $dist 'LICENSE') -Force
    Copy-Item (Join-Path $root 'THIRD-PARTY-NOTICES.txt') (Join-Path $dist 'THIRD-PARTY-NOTICES.txt') -Force
    Copy-Item (Join-Path $root 'README.md') (Join-Path $dist 'README.md') -Force
    Write-Host "Built: $dist\IRQDownloader.exe" -ForegroundColor Green
} finally { Pop-Location }
