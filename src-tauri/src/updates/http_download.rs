//! Скачивание публичного GitHub Release asset.

use crate::core::log::AppLog;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Проверить скачанный zip против digest из GitHub API (`sha256:hex`).
pub fn verify_asset_digest(path: &Path, digest: &str) -> Result<(), String> {
    let d = digest.trim();
    if d.is_empty() {
        return Ok(());
    }
    let hex = d
        .strip_prefix("sha256:")
        .or_else(|| d.strip_prefix("SHA256:"))
        .unwrap_or(d)
        .trim();
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Некорректный digest asset: {d}"));
    }
    let mut file = File::open(path).map_err(|e| format!("Не удалось открыть zip: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("Чтение zip для digest: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let got = format!("{:x}", hasher.finalize());
    if !got.eq_ignore_ascii_case(hex) {
        return Err(format!(
            "SHA-256 обновления не совпал (ожидали {hex}, получили {got})."
        ));
    }
    Ok(())
}

fn github_download_host_ok(url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else {
        return false;
    };
    if u.scheme() != "https" {
        return false;
    }
    let host = u.host_str().unwrap_or("").to_ascii_lowercase();
    host == "api.github.com"
        || host == "github.com"
        || host.ends_with(".github.com")
        || host.ends_with(".githubusercontent.com")
}

/// Скачать asset (публичный browser URL или API octet-stream без токена).
pub async fn download_github_asset(
    url: &str,
    dest: &Path,
    log: Option<&AppLog>,
    mut on_progress: Option<Box<dyn FnMut(f64) + Send>>,
) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("Пустой URL загрузки обновления.".into());
    }
    if !github_download_host_ok(url) {
        return Err("URL загрузки не с GitHub — отказ.".into());
    }

    let is_api_asset = url.contains("api.github.com") && url.contains("/releases/assets/");

    if let Some(l) = log {
        l.info(format!(
            "Загрузка обновления: {}…",
            if is_api_asset {
                "GitHub API asset"
            } else {
                "browser URL"
            }
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent("StatHive")
        .build()
        .map_err(|e| format!("HTTP-клиент: {e}"))?;

    let mut req = client.get(url);
    req = req.header(
        reqwest::header::ACCEPT,
        if is_api_asset {
            "application/octet-stream"
        } else {
            "*/*"
        },
    );

    let res = req.send().await.map_err(|e| {
        let msg = format!("Сеть при загрузке обновления: {e}");
        if let Some(l) = log {
            l.error(&msg);
        }
        msg
    })?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        let hint = body_hint(&body);
        let msg = match status.as_u16() {
            401 | 403 => format!(
                "HTTP {status} — репозиторий должен быть публичным для обновлений. {hint}"
            ),
            404 => format!("HTTP 404 — asset не найден. {hint}"),
            code => format!("HTTP {code} при загрузке обновления. {hint}"),
        };
        if let Some(l) = log {
            l.error(&msg);
        }
        return Err(msg);
    }

    let total = res.content_length().unwrap_or(0);
    if let Some(l) = log {
        if total > 0 {
            l.info(format!(
                "Размер обновления: {:.1} МБ",
                total as f64 / (1024.0 * 1024.0)
            ));
        } else {
            l.info("Размер обновления: неизвестен (нет Content-Length)");
        }
    }

    let mut stream = res.bytes_stream();
    let mut file = File::create(dest).map_err(|e| format!("Не удалось создать файл: {e}"))?;
    let mut done = 0u64;
    let mut last_pct = -1i32;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Обрыв загрузки: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Ошибка записи файла: {e}"))?;
        done += chunk.len() as u64;
        if total > 0 {
            let pct = (done as f64 / total as f64 * 100.0).clamp(0.0, 99.0);
            let rounded = pct as i32;
            if rounded != last_pct {
                last_pct = rounded;
                if let Some(ref mut cb) = on_progress {
                    cb(pct);
                }
            }
        }
    }
    file.flush()
        .map_err(|e| format!("Не удалось сохранить файл: {e}"))?;
    if done == 0 {
        let msg = "Загружен пустой файл обновления (0 байт).";
        if let Some(l) = log {
            l.error(msg);
        }
        return Err(msg.into());
    }
    if let Some(l) = log {
        l.info(format!(
            "Обновление скачано: {done} байт → {}",
            dest.display()
        ));
    }
    Ok(())
}

fn body_hint(body: &str) -> String {
    let t = body.trim();
    if t.is_empty() {
        return String::new();
    }
    let cut: String = t.chars().take(180).collect();
    let safe = cut.replace('\n', " ");
    format!("Ответ: {safe}")
}
