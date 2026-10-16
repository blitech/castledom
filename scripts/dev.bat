@echo off
setlocal EnableExtensions
cd /d "%~dp0.."

rem Trunk 0.21 casse si NO_COLOR=1 (Cursor / CI)
set "NO_COLOR="

set "SOLO=0"
set "DESKTOP=0"
for %%A in (%*) do (
  if /I "%%~A"=="-Solo" set "SOLO=1"
  if /I "%%~A"=="/Solo" set "SOLO=1"
  if /I "%%~A"=="-Desktop" set "DESKTOP=1"
  if /I "%%~A"=="/Desktop" set "DESKTOP=1"
)

if "%SOLO%"=="1" goto :run_app

rem Libère un signaling laissé par un run précédent
taskkill /IM signaling.exe /F >nul 2>&1
taskkill /FI "WINDOWTITLE eq castledom-signaling*" /T /F >nul 2>&1

echo ^>^> signaling  ws://127.0.0.1:3536/castledom_^<CODE^>
start "castledom-signaling" cmd /c "cargo run -p signaling"
timeout /t 1 /nobreak >nul

:run_app
if "%DESKTOP%"=="1" (
  echo ^>^> desktop    cargo run -p game_app --bin castledom
  cargo run -p game_app --bin castledom
) else (
  rem Libère le port 8080 si un trunk précédent tourne encore
  taskkill /IM trunk.exe /F >nul 2>&1
  echo ^>^> web        http://127.0.0.1:8080
  trunk serve --color never
)

if "%SOLO%"=="0" (
  taskkill /FI "WINDOWTITLE eq castledom-signaling*" /T /F >nul 2>&1
  taskkill /IM signaling.exe /F >nul 2>&1
)

endlocal
exit /b %ERRORLEVEL%
