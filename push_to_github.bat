@echo off
setlocal
cd /d "%~dp0"

echo ========================================================
echo   Pushing Network Measurement Studio to GitHub
echo ========================================================
echo Working Dir : %CD%
echo Remote URL  : https://github.com/jaesang-jeong/net_measure_studio.git
echo Branch      : main
echo ========================================================
echo.

:: Prioritize official Windows Git over Cygwin git
if exist "C:\Program Files\Git\cmd\git.exe" (
    set "GIT_EXE=C:\Program Files\Git\cmd\git.exe"
    echo [INFO] Using Windows Git: C:\Program Files\Git\cmd\git.exe
) else if exist "C:\Program Files (x86)\Git\cmd\git.exe" (
    set "GIT_EXE=C:\Program Files (x86)\Git\cmd\git.exe"
    echo [INFO] Using Windows Git: C:\Program Files (x86)\Git\cmd\git.exe
) else (
    set "GIT_EXE=git"
    echo [INFO] Using system PATH git
)

echo.
echo [INFO] Running: "%GIT_EXE%" push -u origin main
echo.

"%GIT_EXE%" push -u origin main

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ========================================================
    echo   [ERROR] Push failed!
    echo ========================================================
    echo 1. If 403 Forbidden error occurred:
    echo    Your Windows Credential Manager has expired GitHub credentials.
    echo    Please sign in when the browser window appears, or update your token.
    echo.
    echo 2. If Repository not found error occurred:
    echo    Please create an empty repository on GitHub:
    echo      - URL : https://github.com/new
    echo      - Name: net_measure_studio
    echo ========================================================
) else (
    echo.
    echo ========================================================
    echo   [SUCCESS] Successfully pushed to GitHub!
    echo ========================================================
)

echo.
pause
