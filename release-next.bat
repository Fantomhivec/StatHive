@echo off
setlocal
cd /d "%~dp0"

REM StatHive release:
REM   bump VERSION everywhere -> commit "vX.Y.Z.N" -> tag vX.Y.Z.N -> push -> GitHub Actions
REM Examples:
REM   release-next.bat
REM   release-next.bat 0.0.0.199
REM   release-next.bat -Yes

echo.
echo === StatHive: release next ===
echo Format: v0.0.0.1 / v0.0.0.199 / v100.2235.626.2626
echo Steps: set versions -^> commit v* -^> tag -^> push -^> Actions (Protect secrets in CI)
echo.

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\release-next.ps1" %*
set ERR=%ERRORLEVEL%
if not %ERR%==0 (
  echo.
  echo ERROR code %ERR%
  pause
  exit /b %ERR%
)
echo.
pause
exit /b 0
