//! Файлы worktree для встроенного редактора. Пути — относительные к корню worktree.
//! Агент правит те же файлы параллельно: запись сверяет время изменения, чтобы не затереть его правки

use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::git::{git, is_repo};

const MAX_SIZE: u64 = 5 << 20;
/// Ошибка записи: файл изменили на диске после того, как редактор его прочитал
const CONFLICT: &str = "conflict";

/// Путь относительно worktree: без абсолютных путей и `..`
fn relative(path: &str) -> Result<&Path, String> {
  let p = Path::new(path);
  if p.components().all(|c| matches!(c, Component::Normal(_))) {
    Ok(p)
  } else {
    Err(format!("недопустимый путь: {path}"))
  }
}

/// Только внутри worktree: без абсолютных путей, `..` и симлинков наружу
fn resolve(root: &str, path: &str) -> Result<PathBuf, String> {
  let full = Path::new(root).join(relative(path)?).canonicalize().map_err(|e| format!("{path}: {e}"))?;
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

/// Новый пустой файл, недостающие папки создаются. Уже есть — ошибка
#[tauri::command]
pub async fn fs_create(root: String, path: String) -> Result<(), String> {
  let full = Path::new(&root).join(relative(&path)?);
  // Ближайшая существующая папка — внутри worktree; всё ниже неё создаём сами, симлинков наружу там нет
  let dir = full.ancestors().skip(1).find(|p| p.is_dir()).and_then(|p| p.strip_prefix(&root).ok());
  resolve(&root, &dir.ok_or(format!("недопустимый путь: {path}"))?.to_string_lossy())?;
  fs::create_dir_all(full.parent().unwrap()).map_err(|e| e.to_string())?;
  fs::File::create_new(&full).map_err(|e| match e.kind() {
    ErrorKind::AlreadyExists => format!("{path} уже существует"),
    _ => format!("{path}: {e}"),
  })?;
  Ok(())
}

/// Файл или папка целиком, безвозвратно. Симлинк удаляется сам, а не то, куда он ведёт
#[tauri::command]
pub async fn fs_delete(root: String, path: String) -> Result<(), String> {
  // Пустой путь — сам worktree: parent у него нет
  let parent = relative(&path)?.parent().ok_or(format!("недопустимый путь: {path}"))?;
  let full = resolve(&root, &parent.to_string_lossy())?.join(Path::new(&path).file_name().unwrap());
  let meta = fs::symlink_metadata(&full).map_err(|e| format!("{path}: {e}"))?;
  if meta.is_dir() { fs::remove_dir_all(&full) } else { fs::remove_file(&full) }.map_err(|e| format!("{path}: {e}"))
}

/// Перетаскивание из Finder: файлы и папки `sources` (абсолютные пути) копируются в папку `dir` worktree.
/// Имя занято — ошибка до копирования, ничего не затираем
#[tauri::command]
pub async fn fs_copy(root: String, dir: String, sources: Vec<String>) -> Result<(), String> {
  let dir = resolve(&root, &dir)?;
  let mut pairs = Vec::new();
  for src in &sources {
    let src = Path::new(src);
    let name = src.file_name().ok_or(format!("недопустимый путь: {}", src.display()))?;
    let dest = dir.join(name);
    if dest.symlink_metadata().is_ok() {
      return Err(format!("{} уже существует", name.to_string_lossy()));
    }
    // cp -R внутрь самого себя копировал бы бесконечно
    if src.canonicalize().is_ok_and(|s| dir.starts_with(s)) {
      return Err(format!("нельзя скопировать {} в саму себя", name.to_string_lossy()));
    }
    pairs.push((src, dest));
  }
  for (src, dest) in pairs {
    // cp -R: папки целиком, симлинки — симлинками, как в Finder
    let out = Command::new("cp").arg("-R").arg(src).arg(&dest).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
      return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
  }
  Ok(())
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
  fn create_copy_delete() {
    let dir = std::env::temp_dir().join(format!("vibecraft-manage-{}", std::process::id()));
    let root = dir.join("wt");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(dir.join("outside")).unwrap();
    std::os::unix::fs::symlink(dir.join("outside"), root.join("out")).unwrap();
    let r = || root.to_string_lossy().into_owned();
    use tauri::async_runtime::block_on as run;

    run(fs_create(r(), "src/new/a.ts".into())).unwrap();
    assert!(root.join("src/new/a.ts").is_file());
    assert!(run(fs_create(r(), "src/new/a.ts".into())).unwrap_err().contains("уже существует"));
    for bad in ["../x", "out/x", "out/deep/x", "", "a/../../x"] {
      assert!(run(fs_create(r(), bad.into())).is_err(), "{bad} должен быть запрещён");
    }
    assert!(!dir.join("outside/deep").exists() && !dir.join("x").exists());

    // Папка и файл из «Finder» — в src; повтор — ошибка, ничего не копируется
    let drop = dir.join("drop");
    fs::create_dir_all(drop.join("assets")).unwrap();
    fs::write(drop.join("assets/logo.svg"), "<svg/>").unwrap();
    fs::write(drop.join("b.txt"), "b").unwrap();
    let sources = || vec![drop.join("assets").to_string_lossy().into_owned(), drop.join("b.txt").to_string_lossy().into_owned()];
    run(fs_copy(r(), "src".into(), sources())).unwrap();
    assert_eq!(fs::read_to_string(root.join("src/assets/logo.svg")).unwrap(), "<svg/>");
    assert!(run(fs_copy(r(), "src".into(), sources())).unwrap_err().contains("уже существует"));
    assert!(run(fs_copy(r(), "out".into(), sources())).is_err(), "в симлинк наружу нельзя");
    assert!(run(fs_copy(r(), "src/assets".into(), vec![root.join("src").to_string_lossy().into_owned()])).is_err());

    // Симлинк удаляется сам, цель остаётся; пустой путь — не весь worktree
    run(fs_delete(r(), "out".into())).unwrap();
    assert!(dir.join("outside").is_dir());
    assert!(run(fs_delete(r(), "".into())).is_err());
    run(fs_delete(r(), "src".into())).unwrap();
    assert!(!root.join("src").exists() && root.is_dir());
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
