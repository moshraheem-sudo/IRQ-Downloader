$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat'
if (-not (Test-Path $vcvars)) { throw "Visual Studio C++ Build Tools were not found: $vcvars" }
Push-Location $root
try {
    Write-Host 'Building IRQ Downloader (Rust release)...' -ForegroundColor Cyan
    & $env:ComSpec /d /s /c "call `"$vcvars`" -arch=x64 -host_arch=x64 >nul && cargo build --workspace --release"
    if ($LASTEXITCODE -ne 0) { throw "Cargo build failed with exit code $LASTEXITCODE" }

    $exe = Join-Path $root 'target\release\IRQDownloader.exe'
    if (-not (Test-Path $exe)) { throw "Release executable not found: $exe" }
    $stage = Join-Path $root "build-cache\stage-$version"
    $portable = Join-Path $root 'dist\IRQ Downloader'
    New-Item -ItemType Directory -Force -Path $stage, (Join-Path $stage 'tools'), (Join-Path $stage 'extension'), (Join-Path $stage 'extension-firefox') | Out-Null
    Copy-Item $exe (Join-Path $stage 'IRQDownloader.exe') -Force
    Copy-Item (Join-Path $root 'tools\*') (Join-Path $stage 'tools') -Recurse -Force
    Copy-Item (Join-Path $root 'extension\*') (Join-Path $stage 'extension') -Recurse -Force
    Copy-Item (Join-Path $root 'extension\*') (Join-Path $stage 'extension-firefox') -Recurse -Force
    Copy-Item (Join-Path $root 'extension\manifest.firefox.json') (Join-Path $stage 'extension-firefox\manifest.json') -Force
    Copy-Item (Join-Path $root 'LICENSE') (Join-Path $stage 'LICENSE') -Force
    Copy-Item (Join-Path $root 'THIRD-PARTY-NOTICES.txt') (Join-Path $stage 'THIRD-PARTY-NOTICES.txt') -Force
    Copy-Item (Join-Path $root 'README.md') (Join-Path $stage 'README.md') -Force
    Copy-Item (Join-Path $root 'DISTRIBUTION_AR.md') (Join-Path $stage 'DISTRIBUTION_AR.md') -Force
    Write-Host "Prepared installer files: $stage" -ForegroundColor Green

    # Keep an already-running portable copy intact; the installer staging above
    # is independent and can still be built while that copy is open.
    try {
        New-Item -ItemType Directory -Force -Path $portable | Out-Null
        Copy-Item (Join-Path $stage 'IRQDownloader.exe') (Join-Path $portable 'IRQDownloader.exe') -Force
        Copy-Item (Join-Path $stage '*') $portable -Recurse -Force
        Write-Host "Portable build: $portable\IRQDownloader.exe" -ForegroundColor Green
    } catch {
        # If the default portable folder is open, keep the current running copy
        # intact and publish this build in a versioned sibling folder instead.
        $versionedPortable = Join-Path (Join-Path $root 'dist') "IRQ Downloader $version"
        New-Item -ItemType Directory -Force -Path $versionedPortable | Out-Null
        Copy-Item (Join-Path $stage '*') $versionedPortable -Recurse -Force
        Write-Warning "The default portable folder may be open. Fresh portable build: $versionedPortable"
    }

    $distRoot = Join-Path $root 'dist'
    New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
    $portableZip = Join-Path $distRoot "IRQDownloader-Portable-$version.zip"
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $portableZip -CompressionLevel Optimal -Force
    Write-Host "Portable package: $portableZip" -ForegroundColor Green

    $iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
    if (-not $iscc) {
        foreach ($candidate in @('C:\Program Files\Inno Setup 7\ISCC.exe', 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe')) {
            if (Test-Path $candidate) { $iscc = $candidate; break }
        }
    }
    if ($iscc) {
        Write-Host 'Building the IRQ Downloader installer...' -ForegroundColor Cyan
        & $iscc "/DStage=$stage" "/DMyAppVersion=$version" (Join-Path $root 'installer\irq_download.iss')
        if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed with exit code $LASTEXITCODE" }
        Write-Host "Installer: $root\dist\IRQDownloader-Setup.exe" -ForegroundColor Green
    } else {
        Write-Warning 'Inno Setup 6 was not found; the executable was built but no setup installer was produced.'
    }
} finally { Pop-Location }
