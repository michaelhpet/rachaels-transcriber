@echo off
setlocal

cd /d "%~dp0"

:: Determine mode
if "%1"=="" (
    python gui.py
) else (
    python transcribe.py %*
)

pause
