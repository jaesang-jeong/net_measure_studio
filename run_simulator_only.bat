@echo off
echo ========================================================
echo   Launching Standalone Target Simulator (Rust Edition)
echo ========================================================
if exist "bin\net_measure_studio.exe" (
    "bin\net_measure_studio.exe" --simulator-only
) else (
    cargo run --release -- --simulator-only
)
pause
