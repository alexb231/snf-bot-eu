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
cd /d "%PROJ_DIR%" || call :fail "Project dir not found: %PROJ_DIR%"

:: === Hardcoded names ===
set "BIN_NAME=sfbot"
set "APP_NAME=sfbot"
echo --- App name: %APP_NAME% (bin: %BIN_NAME%)

:: === Extract version from Cargo.toml ===
for /f "usebackq tokens=2 delims== " %%v in (`findstr /R "^version *= *" Cargo.toml`) do (
    set "ver=%%~v"
)
set "ver=!ver:"=!"
if not defined ver (
    call :fail "Could not read version from Cargo.toml"
)
echo --- Detected version: %ver%

:: === Build Rust (release) ===
echo === Building Rust project (release) ===
cargo build --release || call :fail "Cargo build failed"

:: === Kill local app if running ===
for %%p in ("%APP_NAME%.exe" "%BIN_NAME%.exe") do taskkill /F /IM "%%~p" 2>nul
timeout /t 2 /nobreak >nul

:: === Ensure distribution exe name ===
set "BIN_EXE=%PROJ_DIR%\target\release\%BIN_NAME%.exe"
set "DIST_EXE=%PROJ_DIR%\target\release\%APP_NAME%.exe"
if /I not "%BIN_NAME%"=="%APP_NAME%" (
    copy /Y "%BIN_EXE%" "%DIST_EXE%" >nul || call :fail "Failed to create %APP_NAME%.exe"
)
if not exist "%DIST_EXE%" (
    call :fail "Build output not found: %DIST_EXE%"
)

:: === Copy exe locally ===
set "LOCAL_COPY_DIR=C:\Users\hello\Desktop\sfbot\mains"
if not exist "%LOCAL_COPY_DIR%" (
    call :fail "Local copy dir not found: %LOCAL_COPY_DIR%"
)
echo === Copying %APP_NAME%.exe to mains ===
copy /Y "%DIST_EXE%" "%LOCAL_COPY_DIR%\" || call :fail "Local copy failed"

:: === Create latest.json (UTF-8 without BOM) ===
set "LATEST_JSON=%PROJ_DIR%\latest.json"
set "UPDATE_BASE=https://downloader.sfbot.eu/updates"
set "UPDATE_EXE_NAME=%APP_NAME%.exe"
set "UPDATE_INSTALLER_NAME=%APP_NAME%_installer.exe"
powershell -NoProfile -Command ^
  "$obj = [ordered]@{version='%ver%';notes='Bugfixes and improvements';pub_date=(Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ');platforms=@{'windows-x86_64'=@{url='%UPDATE_BASE%/%UPDATE_EXE_NAME%';installer_url='%UPDATE_BASE%/%UPDATE_INSTALLER_NAME%'}}};$obj | ConvertTo-Json -Depth 5 | Out-File -FilePath '%LATEST_JSON%' -Encoding utf8"

:: === Upload ===
set "BASH=C:\Program Files\Git\bin\bash.exe"
set "REMOTE_USER=ubuntu"
set "REMOTE_HOST=130.61.27.201"
set "REMOTE_DIR=/home/ubuntu/sfrustscript"

echo === Uploading exe ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%DIST_EXE:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/"

echo === Uploading charsToFight.json ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%PROJ_DIR:\=/%/charsToFight.json' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/"

echo === Uploading latest.json ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%LATEST_JSON:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/"

:: === Remote update.sh ===
"%BASH%" -c "ssh -i ~/.ssh/id_ed25519 %REMOTE_USER%@%REMOTE_HOST% 'bash %REMOTE_DIR%/update.sh'"

echo === Deployment complete ===
pause
exit /b 0

:fail
echo [ERROR] %~1
pause
exit /b 1
