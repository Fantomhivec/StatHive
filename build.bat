@echo off
setlocal
cd /d "%~dp0"

REM Local build: StatHive.exe + StatHive.Updater.exe
REM First Rust release build: often 5-15 min. npm should finish in ~30-90 sec.

echo.
echo === StatHive: local build (exe + updater) ===
echo Steps: npm ci -^> tauri build -^> updater -^> zip -^> clean target
echo Tip: thin LTO + CARGO_BUILD_JOBS=1 ^(anti-OOM^). Faster: set UR_CARGO_JOBS=2
echo.

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build.ps1"
set ERR=%ERRORLEVEL%
if not %ERR%==0 (
  echo.
  echo Build failed, code %ERR%
  pause
  exit /b %ERR%
)

echo.
echo Done:
echo   dist-release\StatHive.exe
echo   dist-release\StatHive.Updater.exe
echo   StatHive-win-x64.zip
echo.
pause
exit /b 0
