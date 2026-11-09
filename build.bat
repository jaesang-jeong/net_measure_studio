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

echo [INFO] Compiling Windows resource icon...
if exist "resource.rc" (
    rc.exe /fo app_icon.res resource.rc
    if %ERRORLEVEL% NEQ 0 (
        echo [WARNING] Resource compilation with rc.exe failed, continuing with cargo...
    ) else (
        echo [SUCCESS] Compiled app_icon.res
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

echo [INFO] Copying application icon...
copy /y "nxs.ICO" "bin\nxs.ico" >nul

echo [INFO] Generating and updating Windows shortcuts (.lnk)...
powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $target = (Resolve-Path 'bin\net_measure_studio.exe').Path; $icon = (Resolve-Path 'bin\nxs.ico').Path; $workDir = (Resolve-Path 'bin').Path; $s1 = $ws.CreateShortcut((Join-Path (Get-Location) 'Network Measurement Studio.lnk')); $s1.TargetPath = $target; $s1.WorkingDirectory = $workDir; $s1.IconLocation = $icon; $s1.Description = 'Network Measurement Studio'; $s1.Save(); $s2 = $ws.CreateShortcut((Join-Path (Get-Location) 'bin\Network Measurement Studio.lnk')); $s2.TargetPath = $target; $s2.WorkingDirectory = $workDir; $s2.IconLocation = $icon; $s2.Description = 'Network Measurement Studio'; $s2.Save();"
if %ERRORLEVEL% EQU 0 (
    echo [SUCCESS] Windows shortcuts created with nxs.ico!
)

echo [INFO] Applying code signing signature to executable...
if exist "certs\net_measure_dev.pfx" (
    where signtool.exe >nul 2>nul
    if %ERRORLEVEL% EQU 0 (
        signtool.exe sign /f "certs\net_measure_dev.pfx" /p nxs1234 /fd SHA256 "bin\net_measure_studio.exe" >nul 2>nul
        if %ERRORLEVEL% EQU 0 (
            echo [SUCCESS] Digitally signed bin\net_measure_studio.exe with Nexus Automation certificate!
        )
    )
)

echo [INFO] Removing Mark of the Web (Zone.Identifier) from binaries and shortcuts...
powershell -NoProfile -Command "Get-ChildItem -Path . -Recurse -Include *.exe,*.dll,*.lnk | Unblock-File"

echo ========================================================
echo   Release Build Complete! Output: bin\net_measure_studio.exe
echo ========================================================
