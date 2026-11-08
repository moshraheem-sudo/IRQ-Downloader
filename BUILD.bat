@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build_irq_release.ps1"
if errorlevel 1 pause
