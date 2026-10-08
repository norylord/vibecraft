use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State};

struct Session {
  master: Box<dyn MasterPty + Send>,
  writer: Box<dyn Write + Send>,
  killer: Box<dyn ChildKiller + Send + Sync>,
}

#[derive(Default)]
pub struct Ptys {
  next_id: AtomicU32,
  sessions: Mutex<HashMap<u32, Session>>,
}

fn size(cols: u16, rows: u16) -> PtySize {
  PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }
}

/// Запускает login-shell пользователя: так подтягивается PATH из ~/.zprofile (nvm, brew и т.п.),
/// которого у GUI-приложения на macOS нет. Агенты потом запускаются командой внутри этого shell.
#[tauri::command]
pub fn pty_spawn(
  app: AppHandle,
  state: State<'_, Ptys>,
  cwd: Option<String>,
  cols: u16,
  rows: u16,
  on_data: Channel<InvokeResponseBody>,
  on_exit: Channel<u32>,
) -> Result<u32, String> {
  let pair = native_pty_system().openpty(size(cols, rows)).map_err(|e| e.to_string())?;

  let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
  let mut cmd = CommandBuilder::new(shell);
  cmd.arg("-l");
  if let Some(dir) = cwd.or_else(|| std::env::var("HOME").ok()) {
    cmd.cwd(dir);
  }
  cmd.env("TERM", "xterm-256color");
  cmd.env("COLORTERM", "truecolor");
  // У приложения, запущенного из Finder, LANG пустой — без него ломается UTF-8 в shell
  if std::env::var("LANG").is_err() {
    cmd.env("LANG", "en_US.UTF-8");
  }

  let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
  drop(pair.slave);

  let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
  let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
  let killer = child.clone_killer();

  let id = state.next_id.fetch_add(1, Ordering::Relaxed);
  state.sessions.lock().unwrap().insert(id, Session { master: pair.master, writer, killer });

  // ponytail: шлём каждый read как есть; батчинг (~16 мс) — если xterm захлебнётся на флуде агентов
  std::thread::spawn(move || {
    let mut buf = [0u8; 16 * 1024];
    loop {
      match reader.read(&mut buf) {
        Ok(0) | Err(_) => break,
        Ok(n) => {
          if on_data.send(InvokeResponseBody::Raw(buf[..n].to_vec())).is_err() {
            break;
          }
        }
      }
    }
    let code = child.wait().map(|s| s.exit_code()).unwrap_or(1);
    app.state::<Ptys>().sessions.lock().unwrap().remove(&id);
    let _ = on_exit.send(code);
  });

  Ok(id)
}

#[tauri::command]
pub fn pty_write(state: State<'_, Ptys>, id: u32, data: String) -> Result<(), String> {
  let mut sessions = state.sessions.lock().unwrap();
  let session = sessions.get_mut(&id).ok_or("pty not found")?;
  session.writer.write_all(data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_resize(state: State<'_, Ptys>, id: u32, cols: u16, rows: u16) -> Result<(), String> {
  let sessions = state.sessions.lock().unwrap();
  let session = sessions.get(&id).ok_or("pty not found")?;
  session.master.resize(size(cols, rows)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_kill(state: State<'_, Ptys>, id: u32) {
  if let Some(mut session) = state.sessions.lock().unwrap().remove(&id) {
    let _ = session.killer.kill();
  }
}
