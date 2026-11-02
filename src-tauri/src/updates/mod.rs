mod embedded;
mod http_download;
mod install;

use crate::core::app_version;
use crate::core::log::AppLog;
use serde::{Deserialize, Serialize};

pub use install::download_and_schedule_install;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub body: String,
    pub is_newer: bool,
    #[serde(default)]
    pub current_version: String,
    #[serde(default)]
    pub latest_version: String,
    #[serde(default)]
    pub has_update: bool,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub releases_text: String,
    /// GitHub asset digest (`sha256:…`), если API отдал — проверка после скачивания.
    #[serde(default)]
    pub asset_digest: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    /// API URL asset (public repo тоже отдаёт).
    #[serde(default)]
    url: String,
    /// Прямая ссылка для публичных Releases.
    #[serde(default)]
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    /// e.g. `sha256:abcdef…` (GitHub Releases API).
    #[serde(default)]
    digest: String,
}

pub struct UpdateChecker {
    log: AppLog,
    current_version: String,
}

impl UpdateChecker {
    pub fn new(log: AppLog, current_version: impl Into<String>) -> Self {
        Self {
            log,
            current_version: current_version.into(),
        }
    }

    fn releases_api_base(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}",
            embedded::github_owner(),
            embedded::github_repo()
        )
    }

    /// `verbose`: логи процесса (кнопка «Проверить обновления»).
    /// Тихий режим (авто при входе на профиль / фоновый диалог) — только ошибки.
    pub async fn check(&self, verbose: bool) -> Result<UpdateInfo, String> {
        let current = app_version::normalize(&self.current_version);
        let url = format!("{}/releases/latest", self.releases_api_base());
        if verbose {
            self.log.info("Проверка обновлений…");
        }
        let client = self.http_client();
        let res = client.get(&url).send().await.map_err(|e| {
            let msg = format!("Сеть при проверке обновлений: {e}");
            self.log.error(&msg);
            msg
        })?;
        if res.status().as_u16() == 404 {
            if verbose {
                self.log.info(
                    "releases/latest → 404 (релизов нет) — считаем «актуально».",
                );
            }
            // Quiet auto-check: one API call only. Releases list — button (verbose).
            let releases_text = if verbose {
                self.recent_releases_text(5)
                    .await
                    .unwrap_or_else(|_| "Релизы не найдены.".into())
            } else {
                String::new()
            };
            return Ok(UpdateInfo {
                version: current.clone(),
                download_url: String::new(),
                body: String::new(),
                is_newer: false,
                current_version: current.clone(),
                latest_version: current.clone(),
                has_update: false,
                status: "У вас установлена последняя версия.".into(),
                releases_text,
                asset_digest: String::new(),
            });
        }
        if !res.status().is_success() {
            let code = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            let hint: String = body.chars().take(120).collect();
            let msg = format!(
                "Ошибка проверки обновлений (HTTP {code}){}",
                if hint.trim().is_empty() {
                    String::new()
                } else {
                    format!(": {}", hint.replace('\n', " "))
                }
            );
            // Rate limit / 403: quiet path should not spam error on every remount.
            if verbose || code != 403 {
                self.log.error(&msg);
            } else {
                self.log.warn(&msg);
            }
            return Err(msg);
        }
        let rel: GhRelease = res.json().await.map_err(|e| e.to_string())?;
        let tag = app_version::normalize(&rel.tag_name);
        let preferred = default_update_asset_name();
        let hint = platform_asset_hint();
        if verbose {
            self.log.info(format!(
                "Релиз {tag}: assets = [{}]; ищем «{preferred}» (ОС hint: {hint})",
                rel.assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        // Точное имя ОС; без fallback на «первый zip» чужой платформы.
        let asset = select_release_asset(&rel.assets, preferred, hint);
        let asset_digest = asset
            .map(|a| a.digest.trim().to_string())
            .unwrap_or_default();
        // Публичный репозиторий: browser_download_url, иначе API url.
        let download_url = match asset {
            Some(a) => {
                let browser = a.browser_download_url.trim();
                let api = a.url.trim();
                let chosen = if !browser.is_empty() {
                    browser
                } else {
                    api
                };
                if !chosen.is_empty() {
                    if verbose {
                        self.log.info(format!(
                            "Выбран asset «{}» ({} байт)",
                            a.name, a.size
                        ));
                    }
                    chosen.to_string()
                } else {
                    self.log
                        .error(format!("У asset «{}» нет download URL", a.name));
                    String::new()
                }
            }
            None => {
                self.log.error(format!(
                    "В релизе нет zip для этой ОС («{preferred}» / hint «{hint}»). Не скачиваем чужой asset."
                ));
                String::new()
            }
        };
        let is_newer = app_version::is_newer(&tag, &current);
        if is_newer {
            if verbose {
                self.log.info(format!("Доступна версия {tag}"));
            }
            if download_url.is_empty() {
                self.log.error("Обновление есть, но URL загрузки пустой — установка не удастся.");
            }
        } else if verbose {
            self.log.info("У вас установлена последняя версия.");
        }
        let releases_text = if verbose {
            self.recent_releases_text(5)
                .await
                .unwrap_or_else(|e| format!("Ошибка загрузки релизов: {e}"))
        } else {
            String::new()
        };
        let status = if is_newer {
            format!("Доступна новая версия {tag}.")
        } else {
            "У вас установлена последняя версия.".into()
        };
        Ok(UpdateInfo {
            version: tag.clone(),
            download_url,
            body: rel.body.unwrap_or_default(),
            is_newer,
            current_version: current,
            latest_version: tag,
            has_update: is_newer,
            status,
            releases_text,
            asset_digest,
        })
    }

    fn http_client(&self) -> reqwest::Client {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
        );
        reqwest::Client::builder()
            .user_agent("StatHive")
            .default_headers(h)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }

    async fn recent_releases_text(&self, limit: usize) -> Result<String, String> {
        let url = format!(
            "{}/releases?per_page={limit}",
            self.releases_api_base()
        );
        let client = self.http_client();
        let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status()));
        }
        let list: Vec<GhRelease> = res.json().await.map_err(|e| e.to_string())?;
        if list.is_empty() {
            return Ok("Релизы не найдены.".into());
        }
        let mut out = String::new();
        for r in list.iter().take(limit) {
            let ver = app_version::normalize(&r.tag_name);
            let when = format_published_at(r.published_at.as_deref());
            out.push_str(&format!("v{ver} — {when}\n"));
            let notes = r
                .body
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("Нет описания изменений");
            out.push_str(notes);
            out.push('\n');
            out.push_str(&"-".repeat(40));
            out.push('\n');
        }
        Ok(out.trim_end().to_string())
    }
}

fn default_update_asset_name() -> &'static str {
    #[cfg(windows)]
    {
        "StatHive-win-x64.zip"
    }
    #[cfg(target_os = "linux")]
    {
        "StatHive-linux-x64.zip"
    }
    #[cfg(target_os = "macos")]
    {
        "StatHive-macos-aarch64.zip"
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        "StatHive-win-x64.zip"
    }
}

/// Выбор zip релиза под текущую ОС (паритет SelectReleaseAsset + multi-OS).
fn select_release_asset<'a>(
    assets: &'a [GhAsset],
    preferred: &str,
    hint: &str,
) -> Option<&'a GhAsset> {
    assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(preferred))
        .or_else(|| {
            assets.iter().find(|a| {
                a.name.to_ascii_lowercase().contains(&hint.to_ascii_lowercase())
                    && a.name.to_ascii_lowercase().ends_with(".zip")
            })
        })
}

fn format_published_at(raw: Option<&str>) -> String {
    let Some(s) = raw.map(str::trim).filter(|x| !x.is_empty()) else {
        return "—".into();
    };
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt
            .with_timezone(&chrono::Local)
            .format("%d.%m.%Y %H:%M")
            .to_string();
    }
    s.to_string()
}

fn platform_asset_hint() -> &'static str {
    #[cfg(windows)]
    {
        "win"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        "win"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_os_zip_not_first_foreign() {
        let assets = vec![
            GhAsset {
                name: "StatHive-linux-x64.zip".into(),
                url: "https://api.github.com/repos/o/r/releases/assets/1".into(),
                browser_download_url: String::new(),
                size: 10,
                digest: String::new(),
            },
            GhAsset {
                name: "StatHive-win-x64.zip".into(),
                url: "https://api.github.com/repos/o/r/releases/assets/2".into(),
                browser_download_url: String::new(),
                size: 20,
                digest: String::new(),
            },
        ];
        let a = select_release_asset(&assets, "StatHive-win-x64.zip", "win").unwrap();
        assert_eq!(a.name, "StatHive-win-x64.zip");
        assert!(a.url.ends_with("/2"));
    }

    #[test]
    fn prefers_exact_name_case_insensitive() {
        let assets = vec![GhAsset {
            name: "stathive-win-x64.ZIP".into(),
            url: "https://api.github.com/repos/o/r/releases/assets/9".into(),
            browser_download_url: String::new(),
            size: 1,
            digest: String::new(),
        }];
        let a = select_release_asset(&assets, "StatHive-win-x64.zip", "win").unwrap();
        assert!(a.url.ends_with("/9"));
    }

    #[test]
    fn no_match_returns_none() {
        let assets = vec![GhAsset {
            name: "StatHive-linux-x64.zip".into(),
            url: "https://api.github.com/x".into(),
            browser_download_url: String::new(),
            size: 1,
            digest: String::new(),
        }];
        assert!(select_release_asset(&assets, "StatHive-win-x64.zip", "win").is_none());
    }
}
