use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Каталог рядом с исполняемым файлом (dev: `src-tauri/target/...`, release: папка с .exe).
pub fn exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Каталог конфига в AppData (`StatHive`). Старое имя папки `UniversalRust` переименовывается один раз.
pub fn app_config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let neu = base.join("StatHive");
    let legacy = base.join("UniversalRust");
    if !neu.exists() && legacy.is_dir() {
        match fs::rename(&legacy, &neu) {
            Ok(()) => {}
            Err(_) => {
                // Другой диск / файл занят — копируем верхний уровень как получится.
                let _ = fs::create_dir_all(&neu);
                if let Ok(entries) = fs::read_dir(&legacy) {
                    for e in entries.flatten() {
                        let from = e.path();
                        let to = neu.join(e.file_name());
                        if from.is_file() && !to.exists() {
                            let _ = fs::copy(&from, &to);
                        }
                    }
                }
            }
        }
    }
    neu
}

/// Канонический `.env` — **рядом с exe** (не AppData).
pub fn env_file_path() -> PathBuf {
    purge_stray_env_copies();
    exe_dir().join(".env")
}

/// Файл `*.env` / `.env` только в каталоге exe (имя любое: `keys.env`, `cab.env`, …).
pub fn ensure_env_beside_exe(path: &Path) -> Result<PathBuf, String> {
    let exe = exe_dir();
    let canon_exe = exe.canonicalize().unwrap_or_else(|_| exe.clone());
    let candidate = if path.is_dir() {
        path.join(".env")
    } else {
        path.to_path_buf()
    };
    let name = candidate
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let name_l = name.to_ascii_lowercase();
    if name.is_empty() || !(name_l == ".env" || name_l.ends_with(".env")) {
        return Err(crate::core::i18n::t("errors.env.mustBeEnvExtension"));
    }
    let parent = candidate
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    if !parent.is_dir() {
        return Err(crate::core::i18n::t("errors.env.mustBesideExe"));
    }
    let canon_parent = parent.canonicalize().unwrap_or(parent);
    if canon_parent != canon_exe {
        return Err(crate::core::i18n::t("errors.env.mustBesideExe"));
    }
    Ok(canon_exe.join(name))
}

/// Удалить известные копии `.env` вне папки приложения (exe_dir).
/// Не трогает выбранный пользователем путь и файл рядом с exe.
pub fn purge_stray_env_copies() {
    let home = exe_dir();
    for dir in [app_config_dir(), {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("UniversalRust")
    }] {
        let stray = dir.join(".env");
        if !stray.is_file() {
            continue;
        }
        if is_same_or_under(&stray, &home) {
            continue;
        }
        let _ = fs::remove_file(&stray);
    }
}

fn is_same_or_under(path: &Path, home: &Path) -> bool {
    let Ok(p) = path.canonicalize() else {
        return false;
    };
    let Ok(h) = home.canonicalize() else {
        return false;
    };
    p == h.join(".env") || p.starts_with(&h)
}

