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

cargo build
if errorlevel 1 (
  popd
  exit /b 1
)

popd

taskkill /im sfbot.exe /f >nul 2>&1
call :wait_process_exit sfbot.exe

set "COPY_OK="
call :copy_release "%TAURI_DIR%\target\debug\sfbot.exe" "%OUT_DIR%\sfbot.exe"
if not defined COPY_OK (
  echo Fehler: Kopieren fehlgeschlagen
  exit /b 1
)

echo Fertig.
endlocal
goto :eof

:wait_process_exit
for /l %%i in (1,1,10) do (
  tasklist /fi "imagename eq %~1" | find /i "%~1" >nul || goto :eof
  timeout /t 1 /nobreak >nul
)
goto :eof

:copy_release
for /l %%i in (1,1,5) do (
  if not exist "%~1" (
    timeout /t 1 /nobreak >nul
    goto :continue_copy
  )
  copy /y "%~1" "%~2" >nul
  if not errorlevel 1 (
    set "COPY_OK=1"
    goto :eof
  )
  :continue_copy
  timeout /t 1 /nobreak >nul
)
goto :eof
