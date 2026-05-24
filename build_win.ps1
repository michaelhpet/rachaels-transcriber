$AppName = "RachaelsTranscriber"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "============================================" -ForegroundColor Cyan
Write-Host " Building $AppName for Windows" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $ScriptDir

# Check Python
try {
    python --version | Out-Null
} catch {
    Write-Host "[ERROR] Python not found. Install Python 3.12+ from https://python.org" -ForegroundColor Red
    pause
    exit 1
}

# Download and bundle ffmpeg (cached)
if (-not (Test-Path "ffmpeg\ffmpeg.exe")) {
    Write-Host "Downloading ffmpeg..." -ForegroundColor Yellow
    curl -sL "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" -o ffmpeg.zip
    Expand-Archive -Path ffmpeg.zip -DestinationPath .
    $dir = Get-ChildItem -Directory "ffmpeg-*-essentials_build" | Select-Object -First 1
    if ($dir) {
        New-Item -ItemType Directory -Force -Name ffmpeg | Out-Null
        Move-Item "$dir\bin\ffmpeg.exe" ffmpeg\ -Force
        Move-Item "$dir\bin\ffprobe.exe" ffmpeg\ -Force
        Remove-Item $dir -Recurse -Force
    }
    Remove-Item ffmpeg.zip -Force
} else {
    Write-Host "ffmpeg already cached, skipping download." -ForegroundColor Cyan
}

Write-Host "[1/3] Installing Python dependencies..." -ForegroundColor Green
pip install -r requirements.txt
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] pip install failed." -ForegroundColor Red
    pause
    exit 1
}

Write-Host "[2/2] Installing PyInstaller..." -ForegroundColor Green
pip install pyinstaller
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] PyInstaller install failed." -ForegroundColor Red
    pause
    exit 1
}

# Try to install webrtcvad-wheels for VAD support (optional)
pip install webrtcvad-wheels 2>$null
if ($LASTEXITCODE -ne 0) {
    $extra = "--exclude-module webrtcvad"
} else {
    $extra = "--hidden-import webrtcvad"
}

Write-Host "[3/3] Building executable..." -ForegroundColor Green
pyinstaller `
    --onefile `
    --windowed `
    --name $AppName `
    --add-data "engine.py;." `
    --add-data "assets;assets" `
    --add-data "theme.json;." `
    --add-data "ffmpeg;ffmpeg" `
    --collect-data faster_whisper `
    --hidden-import download_models `
    --additional-hooks-dir hooks `
    @extra `
    --noconfirm `
    --icon assets\icon.ico `
    --hidden-import faster_whisper `
    --hidden-import ctranslate2 `
    --hidden-import pydub `
    gui.py

if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Build failed." -ForegroundColor Red
    pause
    exit 1
}

# Set PE metadata for proper taskbar icon and name
Write-Host "Setting PE metadata..." -ForegroundColor Cyan
curl -sL "https://github.com/electron/rcedit/releases/download/v2.0.0/rcedit-x64.exe" -o rcedit.exe
& .\rcedit.exe "dist\$AppName.exe" --set-version-string "FileDescription" "Rachael's Transcriber" --set-version-string "ProductName" "Rachael's Transcriber" --set-icon "assets\icon.ico"
Remove-Item rcedit.exe

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host " SUCCESS!" -ForegroundColor Green
Write-Host " Executable: dist\$AppName.exe" -ForegroundColor Green
$size = (Get-Item "dist\$AppName.exe").Length / 1MB
Write-Host (" Size: {0:N1} MB" -f $size) -ForegroundColor Green
Write-Host ""
Write-Host " Models are downloaded on first launch (internet required)." -ForegroundColor White
Write-Host " ffmpeg is bundled — no separate install needed." -ForegroundColor White
Write-Host "============================================" -ForegroundColor Green

pause
