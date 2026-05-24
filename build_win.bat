@echo off
setlocal enabledelayedexpansion

set APP_NAME=RachaelsTranscriber
set SCRIPT_DIR=%~dp0

echo ============================================
echo  Building %APP_NAME% for Windows
echo ============================================
echo.

cd /d "%SCRIPT_DIR%"

:: Check for Python
python --version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Python not found. Install Python 3.12+ from https://python.org
    pause
    exit /b 1
)

:: Check for ffmpeg
ffmpeg -version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [WARNING] ffmpeg not found in PATH.
    echo  The .exe will require ffmpeg to be installed separately.
    echo  Install with: choco install ffmpeg  or download from https://ffmpeg.org
    echo.
)

echo [1/3] Installing Python dependencies...
pip install -r requirements.txt
if %ERRORLEVEL% neq 0 (
    echo [ERROR] pip install failed.
    pause
    exit /b 1
)

echo [2/3] Installing PyInstaller...
pip install pyinstaller
if %ERRORLEVEL% neq 0 (
    echo [ERROR] PyInstaller install failed.
    pause
    exit /b 1
)

:: Try to install webrtcvad-wheels for VAD support (optional)
pip install webrtcvad-wheels 2>nul
if errorlevel 1 (
    set EXTRA_PYI=--exclude-module webrtcvad
) else (
    set EXTRA_PYI=--hidden-import webrtcvad
)

echo [3/3] Building executable...
pyinstaller ^
    --onefile ^
    --windowed ^
    --name "%APP_NAME%" ^
    --add-data "engine.py;." ^
    --add-data "assets;assets" ^
    --add-data "theme.json;." ^
    --collect-data faster_whisper ^
    --hidden-import download_models ^
    --additional-hooks-dir hooks ^
    !EXTRA_PYI! ^
    --noconfirm ^
    --icon assets\icon.ico ^
    gui.py

if %ERRORLEVEL% neq 0 (
    echo [ERROR] Build failed.
    pause
    exit /b 1
)

echo.
echo ============================================
echo  SUCCESS!
echo  Executable: dist\%APP_NAME%.exe
echo  Size:
dir "dist\%APP_NAME%.exe" | findstr /i "%APP_NAME%"
echo.
echo  Models are downloaded on first launch.
echo  Users will need ffmpeg + internet for first run.
echo ============================================

pause
