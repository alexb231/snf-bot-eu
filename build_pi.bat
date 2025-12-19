@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

set "SCRIPT_DIR=%~dp0"
set "PROJ_DIR=%SCRIPT_DIR%sfBot\src-tauri"
set "DIST_DIR=%SCRIPT_DIR%dist\pi"
set "CARGO_BIN=%USERPROFILE%\.cargo\bin"
if exist "%CARGO_BIN%\cargo.exe" set "PATH=%CARGO_BIN%;%PATH%"
where rustup >nul 2>&1 || (call :fail "rustup not found. Install rustup or ensure %CARGO_BIN% is in PATH." & exit /b 1)

if not exist "%PROJ_DIR%\Cargo.toml" (
  call :fail "Project dir not found: %PROJ_DIR%"
  exit /b 1
)

set "ZIG_BIN="
set "ZIG_HINT_DIR=C:\Users\hello\Documents\zig-x86_64-windows-0.16.0-dev.1484+d0ba6642b"
if defined ZIG (
  if exist "%ZIG%" set "ZIG_BIN=%ZIG%"
  if exist "%ZIG%\zig.exe" set "ZIG_BIN=%ZIG%\zig.exe"
)
if not defined ZIG_BIN (
  for /f "delims=" %%Z in ('where zig 2^>nul') do set "ZIG_BIN=%%Z"
)
if not defined ZIG_BIN (
  if exist "%ZIG_HINT_DIR%\zig.exe" set "ZIG_BIN=%ZIG_HINT_DIR%\zig.exe"
)
if not defined ZIG_BIN (
  call :fail "zig not found. Install from https://ziglang.org/download/ and add it to PATH or set ZIG=C:\path\to\zig.exe"
  exit /b 1
)
for %%I in ("%ZIG_BIN%") do set "ZIG_DIR=%%~dpI"
set "PATH=%ZIG_DIR%;%PATH%"

where cargo-zigbuild >nul 2>&1 || (call :fail "cargo-zigbuild not found. Run: cargo install cargo-zigbuild" & exit /b 1)

if "%~1"=="" (
  set "TARGETS=arm64 armv7 x64 x86"
) else (
  set "TARGETS=%~1"
)

for %%T in (%TARGETS%) do call :build_one %%T || exit /b 1

echo Done.
exit /b 0

:build_one
set "ARCH=%~1"
set "TARGET="
if /I "%ARCH%"=="arm64" set "TARGET=aarch64-unknown-linux-gnu"
if /I "%ARCH%"=="armv8" set "TARGET=aarch64-unknown-linux-gnu"
if /I "%ARCH%"=="armv7" set "TARGET=armv7-unknown-linux-gnueabihf"
if /I "%ARCH%"=="x64" set "TARGET=x86_64-unknown-linux-gnu"
if /I "%ARCH%"=="x86_64" set "TARGET=x86_64-unknown-linux-gnu"
if /I "%ARCH%"=="amd64" set "TARGET=x86_64-unknown-linux-gnu"
if /I "%ARCH%"=="x86" set "TARGET=i686-unknown-linux-gnu"
if /I "%ARCH%"=="i686" set "TARGET=i686-unknown-linux-gnu"
if not defined TARGET (
  call :fail "Unknown arch: %ARCH% (use arm64 or armv7)"
  exit /b 1
)

echo === Building %TARGET% ===
pushd "%PROJ_DIR%" || (call :fail "Could not cd to %PROJ_DIR%" & exit /b 1)
rustup target add %TARGET% >nul 2>&1 || (popd & call :fail "rustup target add failed for %TARGET%" & exit /b 1)
cargo zigbuild --release --target %TARGET% || (popd & call :fail "Build failed for %TARGET%" & exit /b 1)
popd

set "BIN=%PROJ_DIR%\target\%TARGET%\release\sfbot"
if not exist "%BIN%" (
  call :fail "Binary not found: %BIN%"
  exit /b 1
)

if not exist "%DIST_DIR%" mkdir "%DIST_DIR%" || (call :fail "Could not create %DIST_DIR%" & exit /b 1)
copy /Y "%BIN%" "%DIST_DIR%\sfbot-%TARGET%" >nul || (call :fail "Copy failed for %TARGET%" & exit /b 1)
exit /b 0

:fail
echo [ERROR] %~1
exit /b 1
