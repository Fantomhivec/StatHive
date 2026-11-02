//! Загрузка в UI (прогресс) + тихий скрипт замены файлов.
//! Без окна cmd; старый exe → `PreviousVersion/` (откат при сбое новой версии).

use crate::core::log::AppLog;
use crate::updates::http_download::download_github_asset;
use crate::updates::UpdateInfo;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub async fn download_and_schedule_install(
    app: &AppHandle,
    info: &UpdateInfo,
    log: &AppLog,
) -> Result<String, String> {
    if info.download_url.is_empty() {
        let msg = "Нет URL загрузки (в релизе нет подходящего zip или не выбран asset).";
        log.error(msg);
        return Err(msg.into());
    }

    let main_exe = env::current_exe().map_err(|e| e.to_string())?;
    let app_dir = main_exe
        .parent()
        .ok_or_else(|| "Нет каталога приложения".to_string())?
        .to_path_buf();
    let zip_path = env::temp_dir().join(format!("stathive_update_{}.zip", info.version));

    log.info(format!(
        "Подготовка обновления {} → {}",
        info.version,
        zip_path.display()
    ));

    let app_emit = app.clone();
    let progress_cb: Box<dyn FnMut(f64) + Send> = Box::new(move |pct: f64| {
        let _ = app_emit.emit("updates.progress", serde_json::json!({ "percent": pct }));
    });

    // Сбой загрузки → текущая версия не трогается.
    download_github_asset(
        &info.download_url,
        &zip_path,
        Some(log),
        Some(progress_cb),
    )
    .await
    .map_err(|e| {
        log.error(format!("Сбой загрузки обновления: {e}"));
        e
    })?;

    if let Err(e) = crate::updates::http_download::verify_asset_digest(
        &zip_path,
        &info.asset_digest,
    ) {
        let _ = fs::remove_file(&zip_path);
        log.error(format!("Проверка целостности обновления: {e}"));
        return Err(e);
    }
    if !info.asset_digest.trim().is_empty() {
        log.info("SHA-256 обновления совпал с digest GitHub.");
    }

    let pid = std::process::id();
    let script = write_replace_script(pid, &zip_path, &app_dir, &main_exe).map_err(|e| {
        log.error(format!("Не удалось создать скрипт установки: {e}"));
        e
    })?;
    log.info(format!("Скрипт замены (тихо): {}", script.display()));
    spawn_replace_script(&script, &app_dir).map_err(|e| {
        log.error(e.clone());
        e
    })?;

    let _ = app.emit("updates.progress", serde_json::json!({ "percent": 100.0 }));
    let msg = "Обновление скачано. Приложение будет перезапущено.".to_string();
    log.info(&msg);
    Ok(msg)
}

fn write_replace_script(
    pid: u32,
    zip: &Path,
    app_dir: &Path,
    main_exe: &Path,
) -> Result<PathBuf, String> {
    let exe_name = main_exe
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("StatHive.exe");

    #[cfg(windows)]
    {
        let script = env::temp_dir().join(format!("ur_update_{pid}.bat"));
        // Тихий bat: без pause; старый exe → PreviousVersion; распаковка во TEMP, затем copy.
        let content = format!(
            r#"@echo off
setlocal enabledelayedexpansion
set PID={pid}
set ZIP={zip}
set TARGET={app_dir}
set APP={app_dir}\{exe_name}
set PREV=%TARGET%\PreviousVersion
set TEMP_DIR=%TEMP%\StatHive_Update_%RANDOM%

set WAIT_COUNT=0
:wait
tasklist /FI "PID eq %PID%" 2>NUL | find "%PID%" >NUL
if not errorlevel 1 (
  set /a WAIT_COUNT+=1
  if !WAIT_COUNT! GEQ 45 (
    taskkill /PID %PID% /F >NUL 2>&1
    timeout /t 2 /nobreak >NUL
    goto extract
  )
  timeout /t 1 /nobreak >NUL
  goto wait
)

:extract
mkdir "%TEMP_DIR%" >NUL 2>&1
powershell -NoProfile -ExecutionPolicy Bypass -Command "Expand-Archive -LiteralPath '%ZIP%' -DestinationPath '%TEMP_DIR%' -Force" >NUL 2>&1
if errorlevel 1 exit /b 1

set SOURCE_DIR=%TEMP_DIR%
if not exist "%TEMP_DIR%\*.exe" (
  for /d %%D in ("%TEMP_DIR%\*") do set "SOURCE_DIR=%%D"
)

if not exist "%PREV%" mkdir "%PREV%"
if exist "%APP%" (
  attrib -r -s -h "%APP%" >NUL 2>&1
  move /Y "%APP%" "%PREV%\{exe_name}" >NUL 2>&1
)
if exist "%TARGET%\StatHive.Updater.exe" (
  move /Y "%TARGET%\StatHive.Updater.exe" "%PREV%\StatHive.Updater.exe" >NUL 2>&1
)
if exist "%TARGET%\VERSION" copy /Y "%TARGET%\VERSION" "%PREV%\VERSION" >NUL 2>&1

xcopy /E /Y /I /H "%SOURCE_DIR%\*" "%TARGET%\" >NUL
if errorlevel 1 exit /b 1
if not exist "%APP%" exit /b 1

rd /s /q "%TEMP_DIR%" >NUL 2>&1
del /F /Q "%ZIP%" >NUL 2>&1
start "" "%APP%"
del /F /Q "%~f0" >NUL 2>&1
exit /b 0
"#,
            zip = zip.display(),
            app_dir = app_dir.display(),
            exe_name = exe_name,
            pid = pid,
        );
        fs::write(&script, content).map_err(|e| e.to_string())?;
        Ok(script)
    }
    #[cfg(not(windows))]
    {
        let script = env::temp_dir().join(format!("ur_update_{pid}.sh"));
        let content = format!(
            r#"#!/bin/sh
set -e
PID={pid}
ZIP="{zip}"
TARGET="{app_dir}"
APP="{app_dir}/{exe_name}"
PREV="$TARGET/PreviousVersion"
TEMP_DIR=$(mktemp -d)

WAIT=0
while kill -0 "$PID" 2>/dev/null; do
  WAIT=$((WAIT+1))
  if [ "$WAIT" -ge 45 ]; then
    kill -9 "$PID" 2>/dev/null || true
    sleep 2
    break
  fi
  sleep 1
done

unzip -o "$ZIP" -d "$TEMP_DIR" >/dev/null
SOURCE_DIR="$TEMP_DIR"
# если zip с одной вложенной папкой
count=$(find "$TEMP_DIR" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')
if [ "$count" = "1" ] && [ -z "$(find "$TEMP_DIR" -maxdepth 1 -name '*.exe' -o -name 'StatHive' -o -name 'stathive' 2>/dev/null | head -1)" ]; then
  SOURCE_DIR=$(find "$TEMP_DIR" -mindepth 1 -maxdepth 1 -type d | head -1)
fi

mkdir -p "$PREV"
if [ -f "$APP" ]; then
  mv -f "$APP" "$PREV/$(basename "$APP")"
fi
if [ -f "$TARGET/StatHive.Updater" ]; then
  mv -f "$TARGET/StatHive.Updater" "$PREV/StatHive.Updater"
fi
if [ -f "$TARGET/VERSION" ]; then
  cp -f "$TARGET/VERSION" "$PREV/VERSION"
fi

cp -a "$SOURCE_DIR"/. "$TARGET"/
rm -rf "$TEMP_DIR"
rm -f "$ZIP"
if [ -x "$APP" ]; then
  "$APP" &
fi
rm -f "$0"
"#,
            zip = zip.display(),
            app_dir = app_dir.display(),
            exe_name = exe_name,
            pid = pid,
        );
        fs::write(&script, content).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).map_err(|e| e.to_string())?;
        }
        Ok(script)
    }
}

fn spawn_replace_script(script: &Path, app_dir: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", &script.display().to_string()])
            .current_dir(app_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Не удалось запустить скрипт обновления: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Command::new("sh")
            .arg(script)
            .current_dir(app_dir)
            .spawn()
            .map_err(|e| format!("Не удалось запустить скрипт обновления: {e}"))?;
        Ok(())
    }
}
