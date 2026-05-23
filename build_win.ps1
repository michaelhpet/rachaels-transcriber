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

# Check ffmpeg
try {
    ffmpeg -version | Out-Null
} catch {
    Write-Host "[WARNING] ffmpeg not found in PATH." -ForegroundColor Yellow
    Write-Host " The .exe will require ffmpeg installed separately." -ForegroundColor Yellow
    Write-Host " Install with: choco install ffmpeg" -ForegroundColor Yellow
    Write-Host ""
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

Write-Host "[4/4] Building executable..." -ForegroundColor Green
pyinstaller `
    --onefile `
    --windowed `
    --name $AppName `
    --add-data "engine.py;." `
    --add-data "assets;assets" `
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
Write-Host " Executable: dist\$AppName.exe" -ForegroundColor Green
$size = (Get-Item "dist\$AppName.exe").Length / 1MB
Write-Host (" Size: {0:N1} MB" -f $size) -ForegroundColor Green
Write-Host ""
Write-Host " Models are downloaded on first launch." -ForegroundColor White
Write-Host " Users need ffmpeg + internet for first run." -ForegroundColor White
Write-Host "============================================" -ForegroundColor Green

pause
