use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::{Map, Value};

/// Состояние интерфейса (проекты, агенты, настройки) — файл в ~/.diogen, общий для dev-сборки и .app:
/// localStorage у них разный (http://localhost:1420 против tauri://localhost)
fn file() -> PathBuf {
  crate::hooks::dir().join("state.json")
}

static WRITE: Mutex<()> = Mutex::new(());

fn read() -> Map<String, Value> {
  fs::read_to_string(file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[tauri::command]
pub async fn state_load() -> Map<String, Value> {
  read()
}

/// Ключ перезаписывается целиком; запись атомарная — через временный файл
#[tauri::command]
pub async fn state_save(key: String, value: Value) -> Result<(), String> {
  let _guard = WRITE.lock().unwrap();
  let mut state = read();
  state.insert(key, value);
  let json = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
  let tmp = file().with_extension("json.tmp");
  fs::create_dir_all(crate::hooks::dir()).map_err(|e| e.to_string())?;
  fs::write(&tmp, json).map_err(|e| e.to_string())?;
  fs::rename(tmp, file()).map_err(|e| e.to_string())
}
