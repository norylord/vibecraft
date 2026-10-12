//! Хост терминалов: тот же бинарник с аргументом `--pty-host`, живёт отдельно от приложения и держит PTY.
//! Закрыли или обновили Vibecraft — агенты работают дальше; новое окно переподключается к ним по ключу
//! вкладки и получает хвост вывода, из которого xterm восстанавливает экран.
//!
//! Протокол по unix-сокету — кадры `[тип u8][длина u32 BE][данные]`.
//! Клиент: `o` открыть (JSON `Open`), дальше `d` ввод, `r` размер (cols, rows по u16), `k` убить.
//! Хост: `p` открыто (cols, rows по u16 + хвост вывода) или `e` ошибка, дальше `o` вывод, `x` выход (код u32).

use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Deserialize;

pub const ARG: &str = "--pty-host";
// Хвоста хватает, чтобы xterm восстановил экран агента и последние экраны истории
const REPLAY: usize = 2 << 20;

/// dev-сборка — свой хост: иначе `tauri dev` перехватил бы агентов работающего Vibecraft.app
pub fn socket() -> PathBuf {
  crate::hooks::dir().join(if cfg!(debug_assertions) { "pty-dev.sock" } else { "pty.sock" })
}

pub fn write_frame(w: &mut impl Write, kind: u8, payload: &[u8]) -> io::Result<()> {
  // Одним write_all: кадры разных потоков не перемешиваются
  let mut buf = Vec::with_capacity(5 + payload.len());
  buf.push(kind);
  buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
  buf.extend_from_slice(payload);
  w.write_all(&buf)
}

pub fn read_frame(r: &mut impl Read) -> io::Result<(u8, Vec<u8>)> {
  let mut head = [0u8; 5];
  r.read_exact(&mut head)?;
  let mut payload = vec![0; u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize];
  r.read_exact(&mut payload)?;
  Ok((head[0], payload))
}

pub fn size_bytes(cols: u16, rows: u16) -> [u8; 4] {
  let [c1, c2] = cols.to_be_bytes();
  let [r1, r2] = rows.to_be_bytes();
  [c1, c2, r1, r2]
}

fn size(cols: u16, rows: u16) -> PtySize {
  PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }
}

#[derive(Deserialize)]
pub struct Open {
  pub id: String,
  pub cwd: Option<String>,
  /// 0 — размер не менять (переподключение из скрытой вкладки)
  pub cols: u16,
  pub rows: u16,
  /// false — только переподключиться: восстановленная вкладка не должна запускать shell заново
  pub spawn: bool,
}

/// Ввод под своим замком: запись в PTY может ждать, пока агент читает, а вывод в это время должен течь
struct Session {
  writer: Mutex<Box<dyn Write + Send>>,
  master: Mutex<Box<dyn MasterPty + Send>>,
  killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
  out: Mutex<Output>,
}

struct Output {
  replay: VecDeque<u8>,
  // Одно окно на сессию: новое подключение забирает её у старого
  client: Option<UnixStream>,
}

type Sessions = Arc<Mutex<HashMap<String, Arc<Session>>>>;

/// Login-shell пользователя: так подтягивается PATH из ~/.zprofile (nvm, brew и т.п.),
/// которого у GUI-приложения на macOS нет. Агенты потом запускаются командой внутри этого shell
fn spawn(open: &Open, sessions: Sessions) -> io::Result<Arc<Session>> {
  let pair = native_pty_system().openpty(size(open.cols.max(1), open.rows.max(1))).map_err(io::Error::other)?;

  let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
  let mut cmd = CommandBuilder::new(shell);
  cmd.arg("-l");
  if let Some(dir) = open.cwd.clone().or_else(|| std::env::var("HOME").ok()) {
    cmd.cwd(dir);
  }
  cmd.env("TERM", "xterm-256color");
  cmd.env("COLORTERM", "truecolor");
  // Хуки агентов ссылаются на $VIBECRAFT_DIR/status.sh и claude-hooks.json
  cmd.env("VIBECRAFT_DIR", crate::hooks::dir());
  // У приложения, запущенного из Finder, LANG пустой — без него ломается UTF-8 в shell
  if std::env::var("LANG").is_err() {
    cmd.env("LANG", "en_US.UTF-8");
  }

  let mut child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
  drop(pair.slave);
  let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
  let session = Arc::new(Session {
    writer: Mutex::new(pair.master.take_writer().map_err(io::Error::other)?),
    killer: Mutex::new(child.clone_killer()),
    master: Mutex::new(pair.master),
    out: Mutex::new(Output { replay: VecDeque::new(), client: None }),
  });

  let id = open.id.clone();
  let s = session.clone();
  std::thread::spawn(move || {
    let mut buf = [0u8; 16 * 1024];
    while let Ok(n @ 1..) = reader.read(&mut buf) {
      let mut out = s.out.lock().unwrap();
      out.replay.extend(&buf[..n]);
      let extra = out.replay.len().saturating_sub(REPLAY);
      out.replay.drain(..extra);
      if let Some(client) = &mut out.client {
        if write_frame(client, b'o', &buf[..n]).is_err() {
          out.client = None;
        }
      }
    }
    let code = child.wait().map(|s| s.exit_code()).unwrap_or(1);
    sessions.lock().unwrap().remove(&id);
    if let Some(client) = &mut s.out.lock().unwrap().client {
      let _ = write_frame(client, b'x', &code.to_be_bytes());
    }
  });
  Ok(session)
}

fn serve(mut stream: UnixStream, sessions: Sessions) -> io::Result<()> {
  let (kind, payload) = read_frame(&mut stream)?;
  if kind != b'o' {
    return Ok(());
  }
  let open: Open = serde_json::from_slice(&payload).map_err(io::Error::other)?;
  let found = sessions.lock().unwrap().get(&open.id).cloned();
  let session = match found {
    Some(s) => s,
    None if open.spawn => {
      let s = spawn(&open, sessions.clone())?;
      sessions.lock().unwrap().insert(open.id.clone(), s.clone());
      s
    }
    None => return write_frame(&mut stream, b'e', "сессия терминала завершилась".as_bytes()),
  };
  if open.cols > 0 && open.rows > 0 {
    let _ = session.master.lock().unwrap().resize(size(open.cols, open.rows));
  }
  // Медленное окно не должно тормозить агента: не успело принять вывод — отключаем
  stream.set_write_timeout(Some(Duration::from_secs(5)))?;
  {
    // Хвост и подписка под одним замком с потоком вывода: ничего не теряется и не повторяется
    let mut out = session.out.lock().unwrap();
    let pty = session.master.lock().unwrap().get_size().map_err(io::Error::other)?;
    let mut opened = size_bytes(pty.cols, pty.rows).to_vec();
    opened.extend(out.replay.iter());
    write_frame(&mut stream, b'p', &opened)?;
    out.client = Some(stream.try_clone()?);
  }
  while let Ok((kind, payload)) = read_frame(&mut stream) {
    match (kind, payload.as_slice()) {
      (b'd', data) => {
        let _ = session.writer.lock().unwrap().write_all(data);
      }
      (b'r', &[c1, c2, r1, r2]) => {
        let _ = session.master.lock().unwrap().resize(size(u16::from_be_bytes([c1, c2]), u16::from_be_bytes([r1, r2])));
      }
      (b'k', _) => {
        let _ = session.killer.lock().unwrap().kill();
      }
      _ => {}
    }
  }
  // Окно закрылось — сессия живёт дальше и ждёт переподключения
  Ok(())
}

pub fn run() -> io::Result<()> {
  let dir = crate::hooks::dir();
  fs::create_dir_all(&dir)?;
  // Хост один: второй, запущенный параллельно другим окном, сразу выходит
  let lock = File::create(socket().with_extension("lock"))?;
  if lock.try_lock().is_err() {
    return Ok(());
  }
  let path = socket();
  let _ = fs::remove_file(&path);
  let listener = UnixListener::bind(&path)?;
  // Подключившийся может запускать команды от имени пользователя — сокет только для владельца
  fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;

  let sessions: Sessions = Default::default();
  let clients = Arc::new(AtomicUsize::new(0));
  // Без терминалов и окон хост не нужен; заодно так подхватывается новая версия после обновления.
  // ponytail: окно, подключившееся ровно в момент выхода, получит ошибку — клиент повторяет попытку
  let (s, c) = (sessions.clone(), clients.clone());
  std::thread::spawn(move || loop {
    std::thread::sleep(Duration::from_secs(30));
    let all = s.lock().unwrap();
    if all.is_empty() && c.load(Ordering::SeqCst) == 0 {
      std::process::exit(0);
    }
  });

  accept(listener, sessions, clients);
  Ok(())
}

fn accept(listener: UnixListener, sessions: Sessions, clients: Arc<AtomicUsize>) {
  for stream in listener.incoming().flatten() {
    let (sessions, clients) = (sessions.clone(), clients.clone());
    clients.fetch_add(1, Ordering::SeqCst);
    std::thread::spawn(move || {
      let _ = serve(stream, sessions);
      clients.fetch_sub(1, Ordering::SeqCst);
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn open(path: &PathBuf, spawn: bool) -> (UnixStream, u8, Vec<u8>) {
    let mut s = UnixStream::connect(path).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let open = serde_json::json!({ "id": "tab", "cwd": "/tmp", "cols": 100, "rows": 30, "spawn": spawn });
    write_frame(&mut s, b'o', open.to_string().as_bytes()).unwrap();
    let (kind, payload) = read_frame(&mut s).unwrap();
    (s, kind, payload)
  }

  // Читаем вывод, пока в нём не появится needle
  fn wait_for(s: &mut UnixStream, needle: &str) {
    let mut seen = String::new();
    while !seen.contains(needle) {
      let (kind, payload) = read_frame(s).unwrap();
      assert_eq!(kind, b'o', "сессия закрылась раньше, чем появилось {needle:?}");
      seen.push_str(&String::from_utf8_lossy(&payload));
    }
  }

  #[test]
  fn session_survives_reconnect_and_ends_on_kill() {
    let path = std::env::temp_dir().join(format!("vibecraft-pty-{}.sock", std::process::id()));
    let _ = fs::remove_file(&path);
    let listener = UnixListener::bind(&path).unwrap();
    std::thread::spawn(move || accept(listener, Default::default(), Default::default()));

    let (mut s, kind, opened) = open(&path, true);
    assert_eq!((kind, &opened[..4]), (b'p', &size_bytes(100, 30)[..]));
    // Ответ вычисляет shell — так в выводе он не совпадёт с эхом набранной команды
    write_frame(&mut s, b'd', b"echo vibecraft-$((40+2))\r").unwrap();
    wait_for(&mut s, "vibecraft-42");
    drop(s);

    let (mut s, kind, opened) = open(&path, false);
    assert_eq!(kind, b'p');
    assert!(String::from_utf8_lossy(&opened[4..]).contains("vibecraft-42"), "хвост вывода восстанавливается");
    write_frame(&mut s, b'k', &[]).unwrap();
    loop {
      match read_frame(&mut s).unwrap() {
        (b'x', _) => break,
        (b'o', _) => continue,
        (kind, _) => panic!("неожиданный кадр {kind}"),
      }
    }

    // Сессия ушла из хоста — переподключение без spawn получает ошибку
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while open(&path, false).1 != b'e' {
      assert!(std::time::Instant::now() < deadline, "сессия не удалилась после выхода");
      std::thread::sleep(Duration::from_millis(50));
    }
    fs::remove_file(&path).ok();
  }
}
