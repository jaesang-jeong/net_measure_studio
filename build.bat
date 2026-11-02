@echo off
setlocal
echo ========================================================
echo   Building Network Measurement Studio (Release Mode)
echo ========================================================

:: Check for Visual Studio MSVC environment
where link.exe >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [INFO] Setting up MSVC build environment...
    if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
    ) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
    ) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat"
    ) else if exist "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat"
    )
)

echo [INFO] Running cargo build --release...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed!
    pause
    exit /b %ERRORLEVEL%
)

if not exist "bin" mkdir "bin"

if exist "%USERPROFILE%\AppData\Local\Temp\cargo-target-net-measure\release\net_measure_studio.exe" (
    copy /y "%USERPROFILE%\AppData\Local\Temp\cargo-target-net-measure\release\net_measure_studio.exe" "bin\net_measure_studio.exe"
    echo [SUCCESS] Copied to bin\net_measure_studio.exe
) else if exist "target\release\net_measure_studio.exe" (
    copy /y "target\release\net_measure_studio.exe" "bin\net_measure_studio.exe"
    echo [SUCCESS] Copied to bin\net_measure_studio.exe
)

echo ========================================================
echo   Release Build Complete! Output: bin\net_measure_studio.exe
echo ========================================================
