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
set "ROOT_DIR=C:\Users\hello\RustroverProjects\snf"
set "PROJ_DIR=%ROOT_DIR%\sfBot\src-tauri"
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

:: === Build Linux targets (cross) ===
set "BUILD_PI=%ROOT_DIR%\build_pi.bat"
if not exist "%BUILD_PI%" (
    call :fail "build_pi.bat not found: %BUILD_PI%"
)
echo === Building Linux targets (cross) ===
call "%BUILD_PI%" || call :fail "Linux cross-build failed"

:: === Create latest.json (UTF-8 without BOM) ===
set "LATEST_JSON=%PROJ_DIR%\latest.json"
set "UPDATE_BASE=https://downloader.sfbot.eu/updates"
set "UPDATE_EXE_NAME=%APP_NAME%.exe"
set "UPDATE_INSTALLER_NAME=%APP_NAME%_installer.exe"
set "UPDATE_LINUX_X64_NAME=sfbot-linux-x64"
set "UPDATE_LINUX_ARM64_NAME=sfbot-linux-arm64"
set "UPDATE_LINUX_ARMV7_NAME=sfbot-linux-armv7"
set "UPDATE_LINUX_I686_NAME=sfbot-linux-i686"
powershell -NoProfile -Command ^
  "$platforms = [ordered]@{}; $platforms['windows-x86_64'] = [ordered]@{url='%UPDATE_BASE%/%UPDATE_EXE_NAME%';installer_url='%UPDATE_BASE%/%UPDATE_INSTALLER_NAME%'}; $platforms['linux-x86_64'] = [ordered]@{url='%UPDATE_BASE%/%UPDATE_LINUX_X64_NAME%'}; $platforms['linux-aarch64'] = [ordered]@{url='%UPDATE_BASE%/%UPDATE_LINUX_ARM64_NAME%'}; $platforms['linux-armv7'] = [ordered]@{url='%UPDATE_BASE%/%UPDATE_LINUX_ARMV7_NAME%'}; $platforms['linux-i686'] = [ordered]@{url='%UPDATE_BASE%/%UPDATE_LINUX_I686_NAME%'}; $obj = [ordered]@{version='%ver%';notes='Bugfixes and improvements';pub_date=(Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ');platforms=$platforms}; $obj | ConvertTo-Json -Depth 5 | Out-File -FilePath '%LATEST_JSON%' -Encoding utf8"

:: === Upload ===
set "BASH=C:\Program Files\Git\bin\bash.exe"
set "REMOTE_USER=ubuntu"
set "REMOTE_HOST=130.61.27.201"
set "REMOTE_DIR=/home/ubuntu/sfrustscript"

echo === Uploading exe ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%DIST_EXE:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/"

echo === Uploading linux builds ===
set "LINUX_DIST_DIR=%ROOT_DIR%\dist\pi"
set "LINUX_X64=%LINUX_DIST_DIR%\sfbot-x86_64-unknown-linux-gnu"
set "LINUX_ARM64=%LINUX_DIST_DIR%\sfbot-aarch64-unknown-linux-gnu"
set "LINUX_ARMV7=%LINUX_DIST_DIR%\sfbot-armv7-unknown-linux-gnueabihf"
set "LINUX_I686=%LINUX_DIST_DIR%\sfbot-i686-unknown-linux-gnu"
if not exist "%LINUX_X64%" call :fail "Linux x64 build not found: %LINUX_X64%"
if not exist "%LINUX_ARM64%" call :fail "Linux arm64 build not found: %LINUX_ARM64%"
if not exist "%LINUX_ARMV7%" call :fail "Linux armv7 build not found: %LINUX_ARMV7%"
if not exist "%LINUX_I686%" call :fail "Linux i686 build not found: %LINUX_I686%"
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%LINUX_X64:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/%UPDATE_LINUX_X64_NAME%"
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%LINUX_ARM64:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/%UPDATE_LINUX_ARM64_NAME%"
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%LINUX_ARMV7:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/%UPDATE_LINUX_ARMV7_NAME%"
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%LINUX_I686:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/%UPDATE_LINUX_I686_NAME%"

echo === Uploading charsToFight.json ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%PROJ_DIR:\=/%/charsToFight.json' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/"

echo === Uploading latest.json ===
"%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%LATEST_JSON:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/"

echo === Uploading update.sh ===
set "UPDATE_SH=%PROJ_DIR%\update.sh"
if exist "%UPDATE_SH%" (
    "%BASH%" -c "scp -i ~/.ssh/id_ed25519 '%UPDATE_SH:\=/%' %REMOTE_USER%@%REMOTE_HOST%:%REMOTE_DIR%/update.sh"
) else (
    echo [WARN] update.sh not found: %UPDATE_SH%
)

:: === Remote update.sh ===
"%BASH%" -c "ssh -i ~/.ssh/id_ed25519 %REMOTE_USER%@%REMOTE_HOST% 'bash %REMOTE_DIR%/update.sh'"

echo === Deployment complete ===
pause
exit /b 0

:fail
echo [ERROR] %~1
pause
exit /b 1
