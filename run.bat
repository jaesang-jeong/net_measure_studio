@echo off
echo ========================================================
echo   Launching Network Measurement Studio (Rust Edition)
echo ========================================================
if exist "bin\net_measure_studio.exe" (
    start "" "bin\net_measure_studio.exe"
) else (
    cargo run --release
)
