use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use keyring::Entry;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Адрес и токен интеграции лежат одной записью в Keychain: токен не попадает во webview
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

fn entry(id: &str) -> Result<Entry, String> {
  Entry::new("studio.lince.diogen", id).map_err(|e| e.to_string())
}

/// Keychain читаем один раз за запуск: каждое обращение может стоить запроса пароля macOS
static CACHE: LazyLock<Mutex<HashMap<String, Option<Stored>>>> = LazyLock::new(Default::default);

fn load(id: &str) -> Option<Stored> {
  if let Some(cached) = CACHE.lock().unwrap().get(id) {
    return cached.clone();
  }
  let stored = entry(id).ok().and_then(|e| e.get_password().ok()).and_then(|json| serde_json::from_str(&json).ok());
  remember(id, stored.clone());
  stored
}

fn remember(id: &str, stored: Option<Stored>) {
  CACHE.lock().unwrap().insert(id.to_owned(), stored);
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
  let stored = Stored { url, token: token.trim().to_owned() };
  let json = serde_json::to_string(&stored).map_err(|e| e.to_string())?;
  entry(&id)?.set_password(&json).map_err(|e| e.to_string())?;
  remember(&id, Some(stored));
  Ok(())
}

#[tauri::command]
pub async fn integration_delete(id: String) -> Result<(), String> {
  match entry(&id)?.delete_credential() {
    Ok(()) | Err(keyring::Error::NoEntry) => {
      remember(&id, None);
      Ok(())
    }
    Err(e) => Err(e.to_string()),
  }
}

/// JSON-запрос к API интеграции; path — от корня сервиса, например `/api/v4/user`.
/// YouTrack и GitLab оба принимают токен как Bearer
#[tauri::command]
pub async fn api_request(id: String, path: String, method: Option<String>, body: Option<Value>) -> Result<Value, String> {
  let stored = load(&id).filter(|s| !s.token.is_empty()).ok_or("Интеграция не настроена — откройте настройки")?;
  let method = reqwest::Method::from_bytes(method.as_deref().unwrap_or("GET").as_bytes()).map_err(|e| e.to_string())?;
  let mut req = reqwest::Client::new()
    .request(method, format!("{}{path}", stored.url))
    .bearer_auth(&stored.token)
    .header("Accept", "application/json")
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
  serde_json::from_str(&text).map_err(|e| e.to_string())
}
