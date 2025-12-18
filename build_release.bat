@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
set "TAURI_DIR=%SCRIPT_DIR%sfBot\src-tauri"
set "OUT_DIR=C:\Users\hello\Desktop\sfbot\mains"

if not exist "%TAURI_DIR%" (
  echo Fehler: Tauri-Verzeichnis nicht gefunden: %TAURI_DIR%
  exit /b 1
)

if not exist "%OUT_DIR%" (
  mkdir "%OUT_DIR%" || (
    echo Fehler: Konnte Zielverzeichnis nicht erstellen: %OUT_DIR%
    exit /b 1
  )
)

pushd "%TAURI_DIR%" || (
  echo Fehler: Konnte nicht in %TAURI_DIR% wechseln
  exit /b 1
)

cargo build --release
if errorlevel 1 (
  popd
  exit /b 1
)

popd

taskkill /im sfbot.exe /f >nul 2>&1

xcopy /y /i "%TAURI_DIR%\target\release\*" "%OUT_DIR%\" >nul
if errorlevel 1 (
  echo Fehler: Kopieren fehlgeschlagen
  exit /b 1
)

echo Fertig.
endlocal
