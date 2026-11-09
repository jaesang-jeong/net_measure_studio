@echo off
setlocal
echo ========================================================
echo   Installing Network Measurement Studio Trusted Certificate
echo ========================================================
cd /d "%~dp0"

echo [INFO] Registering certificate into TrustedPublisher and Root store...
powershell -NoProfile -Command "Import-Certificate -FilePath 'net_measure_dev.cer' -CertStoreLocation 'Cert:\CurrentUser\TrustedPublisher' -ErrorAction SilentlyContinue; Import-Certificate -FilePath 'net_measure_dev.cer' -CertStoreLocation 'Cert:\CurrentUser\Root' -ErrorAction SilentlyContinue; Write-Host '[SUCCESS] Certificate registered into CurrentUser TrustedPublisher!' -ForegroundColor Green"

echo [INFO] Unblocking binaries and shortcuts...
powershell -NoProfile -Command "Get-ChildItem -Path .. -Recurse -Include *.exe,*.dll,*.lnk | Unblock-File"

echo.
echo ========================================================
echo   Certificate registration complete!
echo   Windows SmartScreen will no longer block this app.
echo ========================================================
pause
