@echo off
setlocal
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0push_to_github.ps1"
if %ERRORLEVEL% NEQ 0 (
    echo.
    pause
)
