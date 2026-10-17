//! Файлы worktree для встроенного редактора. Пути — относительные к корню worktree.
//! Агент правит те же файлы параллельно: запись сверяет время изменения, чтобы не затереть его правки

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::git::{git, is_repo};

const MAX_SIZE: u64 = 5 << 20;
/// Ошибка записи: файл изменили на диске после того, как редактор его прочитал
const CONFLICT: &str = "conflict";

/// Только внутри worktree: без абсолютных путей, `..` и симлинков наружу
fn resolve(root: &str, path: &str) -> Result<PathBuf, String> {
  if !Path::new(path).components().all(|c| matches!(c, Component::Normal(_))) {
    return Err(format!("недопустимый путь: {path}"));
  }
  let full = Path::new(root).join(path).canonicalize().map_err(|e| format!("{path}: {e}"))?;
  let root = Path::new(root).canonicalize().map_err(|e| e.to_string())?;
  if !full.starts_with(&root) {
    return Err(format!("{path} ведёт за пределы worktree"));
  }
  Ok(full)
}

fn mtime(path: &Path) -> Result<u64, String> {
  let modified = fs::metadata(path).and_then(|m| m.modified()).map_err(|e| e.to_string())?;
  Ok(modified.duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis() as u64)
}

/// Без git нет .gitignore: кроме скрытых папок пропускаем типичные папки зависимостей и сборки
const SKIP_DIRS: [&str; 4] = ["node_modules", "target", "dist", "build"];
/// Папкой без git могли выбрать что-то огромное (весь домашний каталог) — дальше не обходим
const MAX_FILES: usize = 50_000;

// ponytail: .gitignore не читаем, список папок захардкожен; читать его, если в дереве станет шумно
fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
  let Ok(entries) = fs::read_dir(dir) else { return };
  for entry in entries.flatten() {
    if out.len() >= MAX_FILES {
      return;
    }
    let path = entry.path();
    // file_type не идёт по симлинкам — циклов не будет
    match entry.file_type() {
      Ok(t) if t.is_dir() => {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_ref()) {
          walk(root, &path, out);
        }
      }
      Ok(t) if t.is_file() => {
        if let Ok(rel) = path.strip_prefix(root) {
          out.push(rel.to_string_lossy().into_owned());
        }
      }
      _ => {}
    }
  }
}

/// Файлы worktree без игнорируемых .gitignore: отслеживаемые и новые. Папка без git — обход диска
#[tauri::command]
pub async fn fs_list(root: String) -> Result<Vec<String>, String> {
  let mut files = Vec::new();
  if is_repo(&root) {
    let out = git(&root, &["ls-files", "-z", "--cached", "--others", "--exclude-standard"])?;
    // --cached показывает и удалённые, но ещё не закоммиченные файлы, а при конфликте слияния — путь трижды
    files.extend(out.split('\0').filter(|f| !f.is_empty() && Path::new(&root).join(f).is_file()).map(str::to_owned));
  } else {
    walk(Path::new(&root), Path::new(&root), &mut files);
  }
  files.sort();
  files.dedup();
  Ok(files)
}

#[derive(Serialize)]
pub struct File {
  text: String,
  mtime: u64,
}

#[tauri::command]
pub async fn fs_read(root: String, path: String) -> Result<File, String> {
  let full = resolve(&root, &path)?;
  if fs::metadata(&full).map_err(|e| e.to_string())?.len() > MAX_SIZE {
    return Err("Файл больше 5 МБ — откройте его во внешнем редакторе".into());
  }
  let bytes = fs::read(&full).map_err(|e| e.to_string())?;
  let text = String::from_utf8(bytes).map_err(|_| "Двоичный файл — откройте его во внешнем редакторе".to_owned())?;
  Ok(File { text, mtime: mtime(&full)? })
}

/// Запись поверх версии с временем `expected`; изменили на диске — ошибка CONFLICT. Без `expected` — перезапись.
/// Возвращает новое время изменения
#[tauri::command]
pub async fn fs_write(root: String, path: String, text: String, expected: Option<u64>) -> Result<u64, String> {
  let full = resolve(&root, &path)?;
  if expected.is_some_and(|m| mtime(&full).ok() != Some(m)) {
    return Err(CONFLICT.into());
  }
  // На месте, а не через временный файл: сохраняются права (исполняемые скрипты)
  fs::write(&full, text).map_err(|e| e.to_string())?;
  mtime(&full)
}

#[derive(Serialize)]
pub struct Stat {
  /// None — файла больше нет
  mtime: Option<u64>,
  /// id версии в HEAD (blob); None — файл ещё не коммитили
  head: Option<String>,
}

/// Опрос открытого файла: правки на диске (агент) и коммиты (сменилась версия в HEAD)
#[tauri::command]
pub async fn fs_stat(root: String, path: String) -> Stat {
  let Ok(full) = resolve(&root, &path) else {
    return Stat { mtime: None, head: None };
  };
  let head = git(&root, &["rev-parse", "--verify", "--quiet", &format!("HEAD:./{path}")]).ok();
  Stat { mtime: mtime(&full).ok(), head: head.map(|h| h.trim().to_owned()) }
}

/// Версия файла в HEAD — с ней редактор сравнивает текст и отмечает изменённые строки
#[tauri::command]
pub async fn fs_head(root: String, path: String) -> Option<String> {
  resolve(&root, &path).ok()?;
  git(&root, &["show", &format!("HEAD:./{path}")]).ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn paths_stay_inside_worktree() {
    let dir = std::env::temp_dir().join(format!("vibecraft-files-{}", std::process::id()));
    let root = dir.join("wt");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/a.txt"), "a").unwrap();
    fs::write(dir.join("secret"), "s").unwrap();
    std::os::unix::fs::symlink(dir.join("secret"), root.join("link")).unwrap();
    let root = root.to_str().unwrap();

    assert!(resolve(root, "src/a.txt").is_ok());
    for bad in ["../secret", "/etc/passwd", "src/../../secret", "link", "./src/a.txt"] {
      assert!(resolve(root, bad).is_err(), "{bad} должен быть запрещён");
    }

    // Файл поменяли после чтения — запись с прежним временем отклоняется
    let file = tauri::async_runtime::block_on(fs_read(root.into(), "src/a.txt".into())).unwrap();
    let stale = Some(file.mtime - 1);
    let err = tauri::async_runtime::block_on(fs_write(root.into(), "src/a.txt".into(), "b".into(), stale));
    assert_eq!(err.unwrap_err(), CONFLICT);
    let ok = tauri::async_runtime::block_on(fs_write(root.into(), "src/a.txt".into(), "b".into(), Some(file.mtime)));
    assert!(ok.is_ok());
    assert_eq!(fs::read_to_string(dir.join("wt/src/a.txt")).unwrap(), "b");
    fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn folder_without_git_is_own_project() {
    let dir = std::env::temp_dir().join(format!("vibecraft-nogit-{}", std::process::id()));
    for f in ["src/a.ts", ".env", "node_modules/x/index.js", ".cache/c"] {
      let path = dir.join(f);
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      fs::write(path, "").unwrap();
    }
    let root = dir.to_string_lossy().into_owned();

    assert_eq!(tauri::async_runtime::block_on(crate::git::git_root(root.clone())).unwrap(), root);
    let files = tauri::async_runtime::block_on(fs_list(root)).unwrap();
    assert_eq!(files, [".env", "src/a.ts"], "без скрытых папок и node_modules");
    fs::remove_dir_all(&dir).unwrap();
  }
}
