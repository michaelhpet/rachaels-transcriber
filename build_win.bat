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

:: Download and bundle ffmpeg (cached)
if not exist ffmpeg\ffmpeg.exe (
    if exist ffmpeg.zip del ffmpeg.zip
    if exist ffmpeg.zip.tmp del ffmpeg.zip.tmp

    echo Downloading ffmpeg (this may take a few minutes)...
    curl -L --progress-bar --max-time 180 --retry 3 --fail "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" -o ffmpeg.zip.tmp
    if %ERRORLEVEL% neq 0 (
        echo [WARNING] Failed to download ffmpeg.
        echo  The .exe will run without bundled ffmpeg (user must install it separately).
        echo  To bundle ffmpeg manually, place ffmpeg.exe and ffprobe.exe in the ffmpeg\ directory and re-run.
        if exist ffmpeg.zip.tmp del ffmpeg.zip.tmp
    ) else (
        move /y ffmpeg.zip.tmp ffmpeg.zip >nul
        echo Extracting ffmpeg...
        tar -xf ffmpeg.zip
        if %ERRORLEVEL% neq 0 (
            echo [WARNING] Failed to extract ffmpeg.zip (file may be corrupt).
            echo  The .exe will run without bundled ffmpeg.
        ) else (
            for /d %%i in (ffmpeg-*-essentials_build) do (
                if not exist ffmpeg mkdir ffmpeg
                move "%%i\bin\ffmpeg.exe" ffmpeg\ >nul
                move "%%i\bin\ffprobe.exe" ffmpeg\ >nul
                rmdir /s /q "%%i"
            )
            if exist ffmpeg\ffmpeg.exe (
                echo ffmpeg bundled successfully.
            ) else (
                echo [WARNING] ffmpeg binary not found after extraction.
                echo  The .exe will run without bundled ffmpeg.
            )
        )
        del ffmpeg.zip
    )
) else (
    echo ffmpeg already cached, skipping download.
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
    --add-data "ffmpeg;ffmpeg" ^
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

:: Set PE metadata for proper taskbar icon and name
echo Setting PE metadata...
curl -sL "https://github.com/electron/rcedit/releases/download/v2.0.0/rcedit-x64.exe" -o rcedit.exe
rcedit.exe "dist\%APP_NAME%.exe" --set-version-string "FileDescription" "Rachael's Transcriber" --set-version-string "ProductName" "Rachael's Transcriber" --set-icon "assets\icon.ico"
del rcedit.exe

echo.
echo ============================================
echo  SUCCESS!
echo  Executable: dist\%APP_NAME%.exe
echo  Size:
dir "dist\%APP_NAME%.exe" | findstr /i "%APP_NAME%"
echo.
echo  Models are downloaded on first launch (internet required).
echo  ffmpeg is bundled — no separate install needed.
echo ============================================

pause
