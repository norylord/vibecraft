use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Адрес и токен интеграции лежат одной записью в ~/.vibecraft/credentials.json: токен не попадает во webview
/// и не может уйти на другой адрес — запросы к API делает только api_request
#[derive(Serialize, Deserialize, Clone)]
struct Stored {
  url: String,
  token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Integration {
  url: String,
  has_token: bool,
}

fn file() -> PathBuf {
  crate::hooks::dir().join("credentials.json")
}

static WRITE: Mutex<()> = Mutex::new(());

fn read() -> BTreeMap<String, Stored> {
  fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn load(id: &str) -> Option<Stored> {
  read().remove(id)
}

/// None — удалить. Запись атомарная через временный файл, права 0600 — читает только владелец
fn store(id: &str, stored: Option<Stored>) -> Result<(), String> {
  let _guard = WRITE.lock().unwrap();
  let mut all = read();
  match stored {
    Some(s) => all.insert(id.to_owned(), s),
    None => all.remove(id),
  };
  let json = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
  fs::create_dir_all(crate::hooks::dir()).map_err(|e| e.to_string())?;
  let tmp = file().with_extension("json.tmp");
  fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(&tmp)
    .and_then(|mut f| f.write_all(json.as_bytes()))
    .map_err(|e| e.to_string())?;
  fs::rename(tmp, file()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn integration_get(id: String) -> Option<Integration> {
  load(&id).map(|s| Integration { url: s.url, has_token: !s.token.is_empty() })
}

/// token: None — оставить сохранённый (поле в форме не трогали)
#[tauri::command]
pub async fn integration_save(id: String, url: String, token: Option<String>) -> Result<(), String> {
  let token = token.or_else(|| load(&id).map(|s| s.token)).unwrap_or_default();
  let url = url.trim().trim_end_matches('/').to_owned();
  store(&id, Some(Stored { url, token: token.trim().to_owned() }))
}

#[tauri::command]
pub async fn integration_delete(id: String) -> Result<(), String> {
  store(&id, None)
}

const GITHUB: &str = "https://github.com";

/// Токен GitHub CLI (`gh auth login`) для хоста адреса: GitHub работает и без настройки в Vibecraft
fn gh_token(url: &str) -> Option<String> {
  let host = url.split("://").nth(1)?.split('/').next()?;
  let out = Command::new("gh")
    .args(["auth", "token", "--hostname", host.strip_prefix("api.").unwrap_or(host)])
    .env("PATH", crate::git::user_path())
    .output()
    .ok()?;
  let token = String::from_utf8_lossy(&out.stdout).trim().to_owned();
  (out.status.success() && !token.is_empty()).then_some(token)
}

/// Сохранённые адрес и токен; у GitHub без токена — токен gh CLI
fn credentials(id: &str) -> Result<Stored, String> {
  let stored = load(id);
  if let Some(s) = stored.as_ref().filter(|s| !s.token.is_empty()) {
    return Ok(s.clone());
  }
  if id != "github" {
    return Err("Интеграция не настроена — откройте настройки".into());
  }
  let url = stored.map(|s| s.url).filter(|u| !u.is_empty()).unwrap_or_else(|| GITHUB.into());
  let token = gh_token(&url).ok_or("GitHub не подключён: выполните gh auth login или укажите токен в настройках")?;
  Ok(Stored { url, token })
}

/// API GitHub — на api.github.com, у Enterprise — /api/v3 на его хосте; в настройках адрес сайта
fn api_base(id: &str, url: &str) -> String {
  match (id, url) {
    ("github", "https://github.com" | "https://api.github.com") => "https://api.github.com".into(),
    ("github", _) if !url.ends_with("/api/v3") => format!("{url}/api/v3"),
    _ => url.into(),
  }
}

/// JSON-запрос к API интеграции; path — от корня API, например `/api/v4/user` у GitLab, `/user` у GitHub.
/// YouTrack, GitLab и GitHub принимают токен как Bearer
#[tauri::command]
pub async fn api_request(id: String, path: String, method: Option<String>, body: Option<Value>) -> Result<Value, String> {
  let stored = credentials(&id)?;
  let method = reqwest::Method::from_bytes(method.as_deref().unwrap_or("GET").as_bytes()).map_err(|e| e.to_string())?;
  let mut req = reqwest::Client::new()
    .request(method, format!("{}{path}", api_base(&id, &stored.url)))
    .bearer_auth(&stored.token)
    .header("Accept", "application/json")
    // Без User-Agent GitHub отвечает 403
    .header("User-Agent", concat!("vibecraft/", env!("CARGO_PKG_VERSION")))
    .timeout(Duration::from_secs(20));
  if let Some(body) = body {
    req = req.json(&body);
  }
  let res = req.send().await.map_err(|e| e.to_string())?;
  let status = res.status();
  let text = res.text().await.map_err(|e| e.to_string())?;
  if !status.is_success() {
    return Err(format!("{status}: {}", text.chars().take(300).collect::<String>()));
  }
  if text.trim().is_empty() {
    return Ok(Value::Null);
  }
  // Не JSON (лог job'а GitLab) — отдаём текстом
  Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn credentials_roundtrip_with_private_file() {
    use std::os::unix::fs::PermissionsExt;
    use tauri::async_runtime::block_on;
    let home = std::env::temp_dir().join(format!("vibecraft-creds-{}", std::process::id()));
    // HOME больше никто из тестов не читает
    unsafe { std::env::set_var("HOME", &home) };

    block_on(integration_save("youtrack".into(), " https://yt.example/ ".into(), Some(" t1 ".into()))).unwrap();
    block_on(integration_save("gitlab".into(), "https://gl.example".into(), Some("t2".into()))).unwrap();
    block_on(integration_save("youtrack".into(), "https://yt2.example".into(), None)).unwrap();
    let yt = load("youtrack").unwrap();
    assert_eq!((yt.url.as_str(), yt.token.as_str()), ("https://yt2.example", "t1"), "без токена — сохранённый остаётся");
    assert_eq!(fs::metadata(file()).unwrap().permissions().mode() & 0o777, 0o600);

    block_on(integration_delete("youtrack".into())).unwrap();
    assert!(load("youtrack").is_none());
    assert_eq!(load("gitlab").unwrap().token, "t2", "удаление не трогает другие интеграции");
    fs::remove_dir_all(&home).ok();
  }

  #[test]
  fn github_api_base() {
    assert_eq!(api_base("github", "https://github.com"), "https://api.github.com");
    assert_eq!(api_base("github", "https://api.github.com"), "https://api.github.com");
    assert_eq!(api_base("github", "https://ghe.corp"), "https://ghe.corp/api/v3");
    assert_eq!(api_base("github", "https://ghe.corp/api/v3"), "https://ghe.corp/api/v3");
    assert_eq!(api_base("gitlab", "https://gitlab.com"), "https://gitlab.com", "остальные — как есть");
  }
}
