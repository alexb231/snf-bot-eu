@echo off
:: === Admin-Check ===
net session >nul 2>&1
if %errorLevel% NEQ 0 (
    powershell -Command "Start-Process '%~f0' -Verb runAs"
    exit /b
)

:: === STEP 1: Navigate to project directory ===
cd /d C:\Users\hello\RustroverProjects\snf\sfBot\src-tauri

echo === Building Rust project ===
cargo build
IF %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Cargo build failed. Exiting.
    exit /b %ERRORLEVEL%
)

echo === Killing local sfbot.exe if running ===
taskkill /F /IM sfbot.exe 2>NUL
timeout /t 2 /nobreak >nul

echo === Copying sfbot.exe to local target directory ===
copy /Y "target\debug\sfbot.exe" "C:\Users\hello\Desktop\sfbot\mains\"