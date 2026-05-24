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
    --onedir `
    --windowed `
    --name $AppName `
    --add-data "engine.py;." `
    --add-data "assets;assets" `
    --add-data "theme.json;." `
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

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host " SUCCESS!" -ForegroundColor Green
Write-Host " Executable: dist\$AppName\$AppName.exe" -ForegroundColor Green
$size = (Get-ChildItem -Recurse "dist\$AppName" | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host (" Total size: {0:N1} MB" -f $size) -ForegroundColor Green
Write-Host ""
Write-Host " Models are downloaded on first launch (internet required)." -ForegroundColor White
Write-Host " ffmpeg is optional (needed only for files longer than 3 min)." -ForegroundColor White
Write-Host "============================================" -ForegroundColor Green

pause
