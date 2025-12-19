@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

:: === Admin-Check ===
net session >nul 2>&1
if %errorlevel% neq 0 (
    powershell -Command "Start-Process '%~f0' -Verb runAs"
    exit /b
)

:: === STEP 1: Navigate to project directory ===
set "PROJ_DIR=C:\Users\hello\RustroverProjects\snf\sfBot\src-tauri"
cd /d "%PROJ_DIR%" || (echo [ERROR] Project dir not found & exit /b 1)

:: === Extract version from Cargo.toml ===
for /f "usebackq tokens=2 delims== " %%v in (`findstr /R "^version *= *" Cargo.toml`) do (
    set "ver=%%~v"
)
set "ver=!ver:"=!"
if not defined ver (
    echo [ERROR] Could not read version from Cargo.toml
    exit /b 1
)
echo --- Detected version: %ver%

:: === Build Rust ===
echo === Building Rust project ===
cargo build || (echo [ERROR] Cargo build failed & exit /b 1)

:: === Clean NSIS installer folder ===
set "NSIS_DIR=%PROJ_DIR%\target\release\bundle\nsis"
if exist "%NSIS_DIR%" del /q "%NSIS_DIR%\sfbot_*_x64-setup.exe"

:: === Build Tauri (creates exe + installer) ===
echo === Building Tauri bundles ===
cargo tauri build || (echo [ERROR] Tauri build failed & exit /b 1)

:: === Kill local sfbot.exe if running ===
taskkill /F /IM sfbot.exe 2>nul
timeout /t 2 /nobreak >nul

:: === Copy exe locally ===
echo === Copying sfbot.exe to mains ===
copy /Y "target\release\sfbot.exe" "C:\Users\hello\Desktop\sfbot\mains\" || exit /b 1

:: === Find NSIS installer ===
set "INSTALLER="
for %%f in ("%NSIS_DIR%\sfbot_*_x64-setup.exe") do set "INSTALLER=%%f"
if defined INSTALLER (
    echo --- Found installer: %INSTALLER%
) else (
    echo [WARN] No installer found
)

:: === Create latest.json (UTF-8 without BOM) ===
set "LATEST_JSON=%PROJ_DIR%\latest.json"
powershell -NoProfile -Command ^
  "$obj = [ordered]@{version='%ver%';notes='Bugfixes and improvements';pub_date=(Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ');platforms=@{'windows-x86_64'=@{url='https://downloader.sfbot.eu/updates/sfbot.exe';installer_url='https://downloader.sfbot.eu/updates/sfbot_installer.exe'}}};$obj | ConvertTo-Json -Depth 5 | Out-File -FilePath '%LATEST_JSON%' -Encoding utf8"

:: === Upload ===
set "BASH=C:\Program Files\Git\bin\bash.exe"

echo === Uploading exe ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 /c/Users/hello/RustroverProjects/snf/sfBot/src-tauri/target/release/sfbot.exe ubuntu@130.61.27.201:/home/ubuntu/sfrustscript/"

if defined INSTALLER (
    "%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%INSTALLER:\=/%' ubuntu@130.61.27.201:/home/ubuntu/sfrustscript/"
)

echo === Uploading charsToFight.json ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 /c/Users/hello/RustroverProjects/snf/sfBot/src-tauri/charsToFight.json ubuntu@130.61.27.201:/home/ubuntu/sfrustscript/"

echo === Uploading latest.json ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%LATEST_JSON:\=/%' ubuntu@130.61.27.201:/home/ubuntu/sfrustscript/"

:: === Remote update.sh ===
"%BASH%" -c "ssh -i ~/.ssh/id_ed25519 ubuntu@130.61.27.201 'bash ~/sfrustscript/update.sh'"

echo === Deployment complete ===
pause
