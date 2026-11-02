//! Emergency fallback updater (если UI-путь недоступен).
//! Основной поток — скачивание в main + тихий скрипт (`updates/install.rs`).
//! Без окна консоли; при успехе — PreviousVersion + замена + рестарт.
//! Args: --url URL --version VER --target-dir DIR --main-exe PATH

#![cfg_attr(windows, windows_subsystem = "windows")]

use futures_util::StreamExt;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| w[1].clone())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let url = match arg_value(&args, "--url") {
        Some(u) => u,
        None => std::process::exit(1),
    };
    let version = arg_value(&args, "--version").unwrap_or_else(|| "?".into());
    let target_dir = PathBuf::from(arg_value(&args, "--target-dir").unwrap_or_else(|| ".".into()));
    let main_exe = PathBuf::from(arg_value(&args, "--main-exe").unwrap_or_default());

    // Дать main время выйти после spawn.
    thread::sleep(Duration::from_secs(2));

    let zip_path = env::temp_dir().join(format!("stathive_update_{version}.zip"));
    // Сбой загрузки → текущая версия не трогается.
    if download_asset(&url, &zip_path).await.is_err() {
        std::process::exit(1);
    }

    if apply_update(&zip_path, &target_dir, &main_exe).is_err() {
        let _ = fs::remove_file(&zip_path);
        std::process::exit(1);
    }
    let _ = fs::remove_file(&zip_path);

    if main_exe.exists() {
        let mut cmd = Command::new(&main_exe);
        #[cfg(windows)]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = cmd.spawn();
    }
}

fn apply_update(zip: &Path, target: &Path, main_exe: &Path) -> Result<(), String> {
    let temp = env::temp_dir().join(format!("StatHive_Update_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    extract_zip(zip, &temp)?;

    let source = find_payload_root(&temp);
    let prev = target.join("PreviousVersion");
    fs::create_dir_all(&prev).map_err(|e| e.to_string())?;

    if main_exe.exists() {
        let name = main_exe.file_name().unwrap_or_default();
        let _ = fs::rename(main_exe, prev.join(name));
    }
    let updater_name = if cfg!(windows) {
        "StatHive.Updater.exe"
    } else {
        "StatHive.Updater"
    };
    let updater_src = target.join(updater_name);
    if updater_src.exists() {
        let _ = fs::rename(&updater_src, prev.join(updater_name));
    }
    let ver = target.join("VERSION");
    if ver.exists() {
        let _ = fs::copy(&ver, prev.join("VERSION"));
    }

    copy_dir_recursive(&source, target)?;
    let _ = fs::remove_dir_all(&temp);
    if !main_exe.exists() {
        // после xcopy путь тот же
        let candidate = target.join(main_exe.file_name().unwrap_or_default());
        if !candidate.exists() {
            return Err("exe не найден после обновления".into());
        }
    }
    Ok(())
}

fn find_payload_root(temp: &Path) -> PathBuf {
    let has_exe = fs::read_dir(temp)
        .ok()
        .map(|rd| {
            rd.filter_map(|e| e.ok()).any(|e| {
                let n = e.file_name().to_string_lossy().to_ascii_lowercase();
                n.ends_with(".exe") || n == "stathive" || n.eq_ignore_ascii_case("StatHive")
            })
        })
        .unwrap_or(false);
    if has_exe {
        return temp.to_path_buf();
    }
    fs::read_dir(temp)
        .ok()
        .and_then(|rd| {
            rd.filter_map(|e| e.ok())
                .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
        })
        .unwrap_or_else(|| temp.to_path_buf())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

async fn download_asset(url: &str, dest: &Path) -> Result<(), String> {
    let is_api = url.contains("api.github.com") && url.contains("/releases/assets/");
    download_http(url, dest, is_api).await
}

async fn download_http(url: &str, dest: &Path, api_asset: bool) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent("StatHive-Updater")
        .build()
        .map_err(|e| e.to_string())?;
    let req = client.get(url).header(
        reqwest::header::ACCEPT,
        if api_asset {
            "application/octet-stream"
        } else {
            "*/*"
        },
    );
    let res = req.send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    let mut stream = res.bytes_stream();
    let mut file = File::create(dest).map_err(|e| e.to_string())?;
    let mut done = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
    }
    if done == 0 {
        return Err("empty download".into());
    }
    Ok(())
}

fn extract_zip(zip: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!(
                    "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
                    zip.display(),
                    dest.display()
                ),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Expand-Archive failed".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let status = Command::new("unzip")
            .args(["-o", &zip.to_string_lossy(), "-d", &dest.to_string_lossy()])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("unzip failed".into());
        }
        Ok(())
    }
}
