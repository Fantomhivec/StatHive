//! Google user OAuth (Desktop): callback `http://localhost:8765/`, токен в AppData.
//! Client id/secret — локальный `oauth_client.json` (не в git).

use crate::core::i18n::t;
use crate::core::oauth_config;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use url::Url;

/// Сколько ждать вкладку Google. Закрытие вкладки само по себе не приходит в приложение.
const OAUTH_WAIT: StdDuration = StdDuration::from_secs(45);

const REDIRECT_URI: &str = "http://localhost:8765/";
const SCOPES: &str = concat!(
    // drive.file: файлы, созданные приложением / открытые через него (без полного Drive).
    "https://www.googleapis.com/auth/spreadsheets ",
    "https://www.googleapis.com/auth/drive.file ",
    "https://www.googleapis.com/auth/userinfo.email openid"
);
/// Sheets/Drive + Cloud Platform — **только** создание SA из приложения (`iam_sa`).
/// Обычный вход и скрипты используют `SCOPES` без cloud-platform.
const SCOPES_WITH_CLOUD: &str = concat!(
    "https://www.googleapis.com/auth/spreadsheets ",
    "https://www.googleapis.com/auth/drive.file ",
    "https://www.googleapis.com/auth/userinfo.email openid ",
    "https://www.googleapis.com/auth/cloud-platform"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenStore {
    access_token: String,
    refresh_token: Option<String>,
    expires_at_unix: i64,
    #[serde(default)]
    email: String,
}

fn token_path() -> PathBuf {
    crate::core::team_profile::active_dir().join("google_oauth_token.json")
}

fn token_mem() -> &'static Mutex<Option<TokenStore>> {
    static MEM: OnceLock<Mutex<Option<TokenStore>>> = OnceLock::new();
    MEM.get_or_init(|| Mutex::new(None))
}

fn http() -> Result<reqwest::Client, String> {
    Ok(super::http::client())
}

fn load_token() -> Option<TokenStore> {
    if let Ok(g) = token_mem().lock() {
        if let Some(t) = g.as_ref() {
            return Some(t.clone());
        }
    }
    let path = token_path();
    let raw = match crate::core::secure_store::read_text_result(&path) {
        Ok(Some(s)) => s,
        Ok(None) => return None,
        // Битый at-rest (сменился ключ keyring) — убрать blob, чтобы «Проверить подключение» записал новый.
        Err(_) => {
            let _ = fs::remove_file(&path);
            invalidate_token_mem();
            return None;
        }
    };
    let t: TokenStore = serde_json::from_str(&raw).ok()?;
    // Sealed → plaintext: не зависеть от keyring при следующем старте.
    if fs::read_to_string(&path)
        .ok()
        .is_some_and(|disk| disk.trim_start().starts_with("STATHIVE_ATREST_V1."))
    {
        let _ = fs::write(
            &path,
            serde_json::to_string_pretty(&t).unwrap_or_else(|_| raw.clone()),
        );
    }
    if let Ok(mut g) = token_mem().lock() {
        *g = Some(t.clone());
    }
    Some(t)
}

fn save_token(t: &TokenStore) -> Result<(), String> {
    let path = token_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(t).map_err(|e| e.to_string())?;
    // Plaintext в AppData команды (как session/SA) — keyring на части ПК сбрасывает ключ.
    fs::write(&path, raw).map_err(|e| e.to_string())?;
    if let Ok(mut g) = token_mem().lock() {
        *g = Some(t.clone());
    }
    Ok(())
}

pub fn has_cached_token() -> bool {
    load_token().is_some_and(|t| !t.access_token.trim().is_empty())
}

/// Только RAM (переключение команды — не трогать файл активного профиля).
pub fn invalidate_token_mem() {
    if let Ok(mut g) = token_mem().lock() {
        *g = None;
    }
}

pub fn clear_cached_token() {
    invalidate_token_mem();
    let _ = fs::remove_file(token_path());
}

pub fn cached_email() -> Option<String> {
    let e = load_token()?.email.trim().to_string();
    if e.is_empty() {
        None
    } else {
        Some(e)
    }
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Не `cmd start`: `&` в query режет команду, а `start ""` даёт ошибку «не удается найти \\».
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| format!("Не удалось открыть браузер: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Не удалось открыть браузер: {e}"))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Не удалось открыть браузер: {e}"))?;
        Ok(())
    }
}

fn wait_auth_code() -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:8765")
        .map_err(|e| format!("Порт 8765 занят (OAuth callback): {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("OAuth callback: {e}"))?;
    let started = Instant::now();
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(pair) => break pair,
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if started.elapsed() >= OAUTH_WAIT {
                    return Err(
                        "Вход Google отменён: вкладка закрыта или доступ не выдан. Повторите вход."
                            .into(),
                    );
                }
                thread::sleep(StdDuration::from_millis(200));
            }
            Err(e) => return Err(format!("OAuth callback: {e}")),
        }
    };
    let _ = stream.set_nonblocking(false);
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or("");
    let path = first.split_whitespace().nth(1).unwrap_or("/");
    let url = Url::parse(&format!("http://localhost{path}")).map_err(|e| e.to_string())?;
    if let Some((_, err)) = url.query_pairs().find(|(k, _)| k == "error") {
        let body = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><html><body><p>Google sign-in was cancelled. You can close this window.</p></body></html>";
        let _ = stream.write_all(body.as_bytes());
        return Err(format!("Вход Google отменён ({err})."));
    }
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| "OAuth: нет code в callback".to_string())?;
    let body = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><html><body><p>Google connected. You can close this window.</p></body></html>";
    let _ = stream.write_all(body.as_bytes());
    Ok(code)
}

async fn exchange_code(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    token_uri: &str,
    code: &str,
) -> Result<TokenStore, String> {
    let form = [
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", REDIRECT_URI),
        ("grant_type", "authorization_code"),
    ];
    let res = client
        .post(token_uri)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(format!("OAuth token exchange: {t}"));
    }
    #[derive(Deserialize)]
    struct Tok {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<i64>,
    }
    let tok: Tok = res.json().await.map_err(|e| e.to_string())?;
    let expires = Utc::now() + Duration::seconds(tok.expires_in.unwrap_or(3600) - 60);
    Ok(TokenStore {
        access_token: tok.access_token,
        refresh_token: tok.refresh_token,
        expires_at_unix: expires.timestamp(),
        email: String::new(),
    })
}

async fn refresh_token(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    token_uri: &str,
    refresh: &str,
) -> Result<TokenStore, String> {
    let form = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh),
        ("grant_type", "refresh_token"),
    ];
    let res = client
        .post(token_uri)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(format!("OAuth refresh: {t}"));
    }
    #[derive(Deserialize)]
    struct Tok {
        access_token: String,
        expires_in: Option<i64>,
        refresh_token: Option<String>,
    }
    let tok: Tok = res.json().await.map_err(|e| e.to_string())?;
    let expires = Utc::now() + Duration::seconds(tok.expires_in.unwrap_or(3600) - 60);
    Ok(TokenStore {
        access_token: tok.access_token,
        refresh_token: Some(tok.refresh_token.unwrap_or_else(|| refresh.to_string())),
        expires_at_unix: expires.timestamp(),
        email: String::new(),
    })
}

async fn fetch_email(client: &reqwest::Client, access: &str) -> String {
    let res = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access)
        .send()
        .await;
    let Ok(res) = res else {
        return String::new();
    };
    if !res.status().is_success() {
        return String::new();
    }
    #[derive(Deserialize)]
    struct Info {
        email: Option<String>,
    }
    res.json::<Info>()
        .await
        .ok()
        .and_then(|i| i.email)
        .unwrap_or_default()
}

static INTERACTIVE_BUSY: OnceLock<AtomicBool> = OnceLock::new();

fn interactive_busy() -> &'static AtomicBool {
    INTERACTIVE_BUSY.get_or_init(|| AtomicBool::new(false))
}

/// Access token; при необходимости — refresh. Браузер только если нет refresh.
pub async fn user_access_token() -> Result<String, String> {
    get_access_token_inner(false, SCOPES, "consent").await
}

/// Интерактивный OAuth (браузер) — Sheets/Drive.
pub async fn authorize_interactive() -> Result<String, String> {
    authorize_interactive_scopes(SCOPES).await
}

/// Сброс сохранённого токена и выбор другого аккаунта в окне Google.
pub async fn authorize_switch_account() -> Result<String, String> {
    clear_cached_token();
    authorize_interactive_scopes_prompt(SCOPES, "select_account consent").await
}

/// OAuth с Cloud Platform (создание SA / IAM). Откроет браузер с доп. согласием.
pub async fn authorize_interactive_cloud() -> Result<String, String> {
    authorize_interactive_scopes(SCOPES_WITH_CLOUD).await
}

async fn authorize_interactive_scopes(scopes: &str) -> Result<String, String> {
    authorize_interactive_scopes_prompt(scopes, "consent").await
}

async fn authorize_interactive_scopes_prompt(scopes: &str, prompt: &str) -> Result<String, String> {
    if interactive_busy()
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("OAuth уже выполняется — дождитесь окна браузера".into());
    }
    let res = get_access_token_inner(true, scopes, prompt).await;
    interactive_busy().store(false, Ordering::SeqCst);
    res
}

/// Тихий прогрев на старте (только refresh).
pub async fn warm_user_token() {
    let _ = get_access_token_inner(false, SCOPES, "consent").await;
}

async fn get_access_token_inner(
    force_interactive: bool,
    scopes: &str,
    prompt: &str,
) -> Result<String, String> {
    let raw = oauth_config::load_raw().ok_or_else(|| {
        t("errors.google.noClientLocal")
    })?;
    let (client_id, client_secret, token_uri) = oauth_config::parse_client_id_secret(&raw)?;
    let client = http()?;

    if !force_interactive {
        if let Some(tok) = load_token() {
            if Utc::now().timestamp() < tok.expires_at_unix - 30 && !tok.access_token.is_empty() {
                return Ok(tok.access_token);
            }
            if let Some(ref rt) = tok.refresh_token {
                match refresh_token(&client, &client_id, &client_secret, &token_uri, rt).await {
                    Ok(mut n) => {
                        n.email = if tok.email.is_empty() {
                            fetch_email(&client, &n.access_token).await
                        } else {
                            tok.email.clone()
                        };
                        save_token(&n)?;
                        return Ok(n.access_token);
                    }
                    Err(_) => {}
                }
            }
            return Err(t("errors.google.sessionExpired"));
        }
        return Err(t("errors.google.notConnectedHint"));
    }

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt={}&include_granted_scopes=true",
        urlencoding::encode(&client_id),
        urlencoding::encode(REDIRECT_URI),
        urlencoding::encode(scopes),
        urlencoding::encode(prompt),
    );
    let code_handle = thread::spawn(wait_auth_code);
    open_browser(&auth_url)?;
    let code = code_handle
        .join()
        .map_err(|_| "OAuth thread panic".to_string())??;
    let mut store = exchange_code(&client, &client_id, &client_secret, &token_uri, &code).await?;
    store.email = fetch_email(&client, &store.access_token).await;
    save_token(&store)?;
    Ok(store.access_token)
}
