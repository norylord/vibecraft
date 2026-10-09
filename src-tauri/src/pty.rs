use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State};

use crate::ptyhost::{self, read_frame, size_bytes, write_frame};

/// Подключения к сессиям хоста по ключу вкладки
#[derive(Default)]
pub struct Ptys(Mutex<HashMap<String, UnixStream>>);

/// Хост терминалов (см. ptyhost.rs); нет его — запускаем. Своя группа процессов:
/// Ctrl+C в терминале `tauri dev` не должен убить агентов
fn connect() -> Result<UnixStream, String> {
  if let Ok(stream) = UnixStream::connect(ptyhost::socket()) {
    return Ok(stream);
  }
  let exe = std::env::current_exe().map_err(|e| e.to_string())?;
  let mut host = Command::new(exe)
    .arg(ptyhost::ARG)
    .process_group(0)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .map_err(|e| e.to_string())?;
  // Ждём в фоне: иначе вышедший хост висел бы зомби до закрытия приложения
  std::thread::spawn(move || host.wait());
  for _ in 0..100 {
    std::thread::sleep(Duration::from_millis(20));
    if let Ok(stream) = UnixStream::connect(ptyhost::socket()) {
      return Ok(stream);
    }
  }
  Err("не удалось запустить хост терминалов".into())
}

/// Ok(None) — сессии нет (восстановленная вкладка, а агент завершился, пока приложение было закрыто)
fn handshake(open: &serde_json::Value) -> Result<Option<(UnixStream, Vec<u8>)>, String> {
  let mut stream = connect()?;
  write_frame(&mut stream, b'o', open.to_string().as_bytes()).map_err(|e| e.to_string())?;
  match read_frame(&mut stream).map_err(|e| e.to_string())? {
    (b'p', opened) => Ok(Some((stream, opened))),
    _ => Ok(None),
  }
}

/// Открывает вкладку в хосте: spawn — новый login-shell, иначе только переподключение.
/// Первое сообщение в on_data — размер PTY (cols, rows по u16) и хвост вывода для восстановления экрана.
/// cols/rows = 0 — размер не менять (вкладка скрыта и ещё не измерена)
#[tauri::command]
pub async fn pty_open(
  app: AppHandle,
  key: String,
  cwd: Option<String>,
  cols: u16,
  rows: u16,
  spawn: bool,
  on_data: Channel<InvokeResponseBody>,
  on_exit: Channel<u32>,
) -> Result<bool, String> {
  let open = serde_json::json!({ "id": key, "cwd": cwd, "cols": cols, "rows": rows, "spawn": spawn });
  // Хост мог выйти ровно в момент подключения (см. ptyhost::run) — вторая попытка запустит новый
  let Some((mut stream, opened)) = handshake(&open).or_else(|_| handshake(&open))? else {
    return Ok(false);
  };
  let writer = stream.try_clone().map_err(|e| e.to_string())?;
  app.state::<Ptys>().0.lock().unwrap().insert(key.clone(), writer);
  on_data.send(InvokeResponseBody::Raw(opened)).map_err(|e| e.to_string())?;

  std::thread::spawn(move || {
    // Обрыв без кадра выхода — хост упал: для вкладки это тоже конец
    let mut code = 1;
    while let Ok((kind, payload)) = read_frame(&mut stream) {
      match (kind, payload.as_slice()) {
        (b'o', _) => {
          if on_data.send(InvokeResponseBody::Raw(payload)).is_err() {
            break;
          }
        }
        (b'x', &[a, b, c, d]) => {
          code = u32::from_be_bytes([a, b, c, d]);
          break;
        }
        _ => {}
      }
    }
    app.state::<Ptys>().0.lock().unwrap().remove(&key);
    let _ = on_exit.send(code);
  });
  Ok(true)
}

fn send(state: &Ptys, key: &str, kind: u8, payload: &[u8]) -> Result<(), String> {
  let mut streams = state.0.lock().unwrap();
  let stream = streams.get_mut(key).ok_or("pty not found")?;
  write_frame(stream, kind, payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_write(state: State<'_, Ptys>, key: String, data: String) -> Result<(), String> {
  send(&state, &key, b'd', data.as_bytes())
}

#[tauri::command]
pub fn pty_resize(state: State<'_, Ptys>, key: String, cols: u16, rows: u16) -> Result<(), String> {
  send(&state, &key, b'r', &size_bytes(cols, rows))
}

/// Закрытие вкладки — агент завершается; закрытие приложения — нет: сессии остаются в хосте
#[tauri::command]
pub fn pty_kill(state: State<'_, Ptys>, key: String) {
  let _ = send(&state, &key, b'k', &[]);
}
