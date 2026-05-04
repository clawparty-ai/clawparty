@echo off
setlocal

echo Starting ClawParty services...
echo.

set "DATA_DIR=%USERPROFILE%\.clawparty"
set "ZC_DIR=%DATA_DIR%\.zeroclaw"
if not exist "%ZC_DIR%" mkdir "%ZC_DIR%"

:: Log directory
set "LOG_DIR=%DATA_DIR%\logs"
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"

:: -------------------------------------------------------------
:: 0) Seed initial config.toml if zeroclaw dir has none
:: -------------------------------------------------------------
if not exist "%ZC_DIR%\config.toml" (
    if exist "%~dp0config.toml" (
        echo [SETUP] Copying bundled config.toml to %ZC_DIR% ...
        copy /Y "%~dp0config.toml" "%ZC_DIR%\config.toml" >nul
        echo [SETUP] config.toml copied.
    ) else (
        echo [SETUP] No bundled config.toml found in script directory.
    )
)

:: ---------------------------------------------------------------
:: 1) Start ZeroClaw daemon (port 42617)
:: ---------------------------------------------------------------
echo [1/2] Starting ZeroClaw daemon on port 42617...
start /B "" "%~dp0zeroclaw.exe" daemon --port 42617 --config-dir "%ZC_DIR%" >"%LOG_DIR%\zeroclaw.log" 2>&1
echo ZeroClaw daemon started.

:: Brief pause before launching ZTM
ping -n 2 127.0.0.1 >nul

:: ---------------------------------------------------------------
:: 2) Start ZTM agent (port 6789, embedded GUI)
:: ---------------------------------------------------------------
echo [2/2] Starting ZTM agent on http://127.0.0.1:6789 ...
start /B "" "%~dp0ztm.exe" run agent --listen 127.0.0.1:6789 --data "%DATA_DIR%" --api-token enjoy-party >"%LOG_DIR%\ztm.log" 2>&1
echo ZTM agent started.

:: ---------------------------------------------------------------
echo.
echo All services launched in background!
echo.
echo   ZeroClaw Gateway: http://localhost:42617
echo   ZTM Agent API:    http://127.0.0.1:6789
echo   Web GUI:          http://127.0.0.1:6789/gui/
echo.
echo Logs: %LOG_DIR%
echo Run stop.bat to stop all services.

endlocal
