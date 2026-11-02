@echo off
setlocal
echo ========================================================
echo   Pushing Network Measurement Studio to GitHub
echo ========================================================
echo Remote URL: https://github.com/jaesang-jeong/net_measure_studio.git
echo Branch    : main
echo.
git push -u origin main
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Push failed!
    echo If the repository has not been created on GitHub yet, please create a new repository:
    echo   1. Go to https://github.com/new
    echo   2. Repository name: net_measure_studio
    echo   3. Do NOT initialize with README/license (keep it empty)
    echo   4. Click 'Create repository', then run this script again!
    echo.
) else (
    echo.
    echo [SUCCESS] Successfully pushed to GitHub!
)
pause
