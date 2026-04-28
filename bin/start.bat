@echo off
setlocal

:: Start ZeroClaw daemon in background
start "ZeroClaw" "%~dp0zeroclaw.exe" daemon --port 42617 --config-dir "%USERPROFILE%\.clawparty\.zeroclaw"

:: Wait a moment for ZeroClaw to start
timeout /t 2 /nobreak >nul

:: Run ZTM
"%~dp0ztm.exe"
