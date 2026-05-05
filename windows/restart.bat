@echo off
echo Restarting ClawParty services...
echo.

:: Ensure we run from the same directory as this script
cd /d "%~dp0"

:: 1) Stop all services
call "%~dp0stop.bat"

:: Brief pause to let processes fully terminate
ping -n 3 127.0.0.1 >nul

:: 2) Start all services
call "%~dp0start.bat"
