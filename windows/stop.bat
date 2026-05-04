@echo off
echo Stopping ClawParty services...

:: Stop ZTM agent
echo [1/2] Stopping ZTM agent...
taskkill /F /IM ztm.exe > nul 2>&1
if %errorlevel% equ 0 (
    echo ZTM agent stopped.
) else (
    echo ZTM agent was not running.
)

:: Stop ZeroClaw daemon
echo [2/2] Stopping ZeroClaw daemon...
taskkill /F /IM zeroclaw.exe > nul 2>&1
if %errorlevel% equ 0 (
    echo ZeroClaw daemon stopped.
) else (
    echo ZeroClaw daemon was not running.
)

echo.
echo All services stopped.
