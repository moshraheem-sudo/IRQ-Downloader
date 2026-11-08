@echo off
setlocal
cd /d "%~dp0"
set "APP_EXE=%~dp0dist\IRQ Downloader\IRQDownloader.exe"
if not exist "%APP_EXE%" (
  echo IRQ Downloader is not built yet.
  echo Run App\BUILD.bat first, then launch this file again.
  pause
  exit /b 1
)
start "IRQ Downloader" "%APP_EXE%" %*
