use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::Duration;

use serde::Serialize;

/// PATH из login-shell пользователя. Приложение из Dock получает урезанный PATH,
/// и git-хуки (husky, lint-staged) не находят node. Считается один раз
pub fn user_path() -> &'static str {
  static PATH: OnceLock<String> = OnceLock::new();
  PATH.get_or_init(|| {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
      // -i: nvm и т.п. настраивают PATH в .zshrc; маркер отрезает вывод rc-файлов
      let out = Command::new(shell)
        .args(["-l", "-i", "-c", "printf '__DIOGEN_PATH__%s' \"$PATH\""])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
      let path = out.ok().and_then(|o| {
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        text.rsplit_once("__DIOGEN_PATH__").map(|(_, p)| p.trim().to_owned())
      });
      let _ = tx.send(path.filter(|p| !p.is_empty()));
    });
    // Зависший rc-файл не должен навсегда заблокировать git
    rx.recv_timeout(Duration::from_secs(3)).ok().flatten().unwrap_or_else(|| std::env::var("PATH").unwrap_or_default())
  })
}

fn git_cmd() -> Command {
  let mut cmd = Command::new("git");
  cmd.env("PATH", user_path());
  cmd
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Worktree {
  path: String,
  /// None — detached HEAD
  branch: Option<String>,
}

fn git(dir: &str, args: &[&str]) -> Result<String, String> {
  let out = git_cmd()
    .arg("-C")
    .arg(dir)
    .args(args)
    .output()
    .map_err(|e| e.to_string())?;
  if out.status.success() {
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
  } else {
    Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
  }
}

fn parse_worktrees(porcelain: &str) -> Vec<Worktree> {
  porcelain
    .split("\n\n")
    .filter_map(|block| {
      let mut path = None;
      let mut branch = None;
      for line in block.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
          path = Some(p.to_owned());
        } else if let Some(b) = line.strip_prefix("branch ") {
          branch = Some(b.trim_start_matches("refs/heads/").to_owned());
        } else if line == "bare" {
          return None;
        }
      }
      Some(Worktree { path: path?, branch })
    })
    .collect()
}

/// Корень репозитория для любой папки внутри него
#[tauri::command]
pub async fn git_root(path: String) -> Result<String, String> {
  Ok(git(&path, &["rev-parse", "--show-toplevel"])?.trim().to_owned())
}

/// Первым всегда идёт основной worktree репозитория
#[tauri::command]
pub async fn git_worktrees(repo: String) -> Result<Vec<Worktree>, String> {
  Ok(parse_worktrees(&git(&repo, &["worktree", "list", "--porcelain"])?))
}

/// Новый worktree в ~/.diogen/worktrees/<repo>/<branch>; существующую ветку переиспользует
#[tauri::command]
pub async fn git_worktree_add(repo: String, branch: String) -> Result<String, String> {
  let home = std::env::var("HOME").map_err(|e| e.to_string())?;
  let repo_name = Path::new(&repo).file_name().and_then(|n| n.to_str()).unwrap_or("repo");
  let path = format!("{home}/.diogen/worktrees/{repo_name}/{}", branch.replace('/', "-"));

  let exists = git(&repo, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
  if exists {
    git(&repo, &["worktree", "add", &path, &branch])?;
  } else {
    git(&repo, &["worktree", "add", "-b", &branch, &path])?;
  }
  Ok(path)
}

/// Unified diff всего, что сделано в worktree: от merge-base с base (коммиты ветки + незакоммиченное)
/// или от HEAD, если base нет. Новые файлы git diff не видит — дописываем их через --no-index.
/// quotePath=false: иначе кириллица в путях превращается в восьмеричные escape-коды
#[tauri::command]
pub async fn git_diff(path: String, base: Option<String>) -> Result<String, String> {
  const Q: [&str; 2] = ["-c", "core.quotePath=false"];
  let mut out = match &base {
    Some(base) => git(&path, &[Q[0], Q[1], "diff", "--merge-base", base])?,
    None => git(&path, &[Q[0], Q[1], "diff", "HEAD"])?,
  };
  let untracked = git(&path, &["ls-files", "--others", "--exclude-standard", "-z"])?;
  for file in untracked.split('\0').filter(|f| !f.is_empty()) {
    // --no-index завершается с кодом 1, когда файлы различаются, — поэтому без хелпера git()
    let o = git_cmd()
      .args(["-C", &path, Q[0], Q[1], "diff", "--no-index", "--", "/dev/null", file])
      .output()
      .map_err(|e| e.to_string())?;
    out.push_str(&String::from_utf8_lossy(&o.stdout));
  }
  Ok(out)
}

#[derive(Serialize)]
pub struct Status {
  /// Незакоммиченные изменения, включая новые файлы
  changes: usize,
  /// Коммитов впереди upstream; None — ветку ещё не публиковали
  ahead: Option<u32>,
  /// Куда публиковать новую ветку; None — у репозитория нет remote
  remote: Option<String>,
}

fn ahead(path: &str) -> Option<u32> {
  git(path, &["rev-list", "--count", "@{u}..HEAD"]).ok()?.trim().parse().ok()
}

/// origin, если есть, иначе первый remote репозитория
fn push_remote(path: &str) -> Option<String> {
  let remotes = git(path, &["remote"]).ok()?;
  let remotes: Vec<&str> = remotes.lines().collect();
  let remote = if remotes.contains(&"origin") { Some("origin") } else { remotes.first().copied() };
  remote.map(str::to_owned)
}

/// URL того remote, куда пушим: по нему фронтенд находит проект в GitLab
#[tauri::command]
pub async fn git_remote_url(path: String) -> Option<String> {
  let remote = push_remote(&path)?;
  git(&path, &["remote", "get-url", &remote]).ok().map(|s| s.trim().to_owned())
}

/// Клон в <dir>/<name>. Если там уже этот же репозиторий — просто возвращаем путь
#[tauri::command]
pub async fn git_clone(url: String, dir: String, name: String) -> Result<String, String> {
  if name.is_empty() || name.contains('/') || name.starts_with('.') {
    return Err(format!("недопустимое имя папки: {name}"));
  }
  let path = Path::new(&dir).join(&name).to_string_lossy().into_owned();
  if Path::new(&path).exists() {
    let same = git(&path, &["remote", "get-url", "origin"]).is_ok_and(|u| u.trim() == url);
    return if same { Ok(path) } else { Err(format!("Папка {path} уже занята другим содержимым")) };
  }
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  git(&dir, &["clone", &url, &name])?;
  Ok(path)
}

#[tauri::command]
pub async fn git_status(path: String) -> Result<Status, String> {
  let changes = git(&path, &["status", "--porcelain"])?.lines().count();
  Ok(Status { changes, ahead: ahead(&path), remote: push_remote(&path) })
}

// ponytail: git-хуки (husky, lint-staged) берут PATH процесса — в dev он из терминала,
// в собранном .app урезан; чинить при сборке (fix-path-env)
#[tauri::command]
pub async fn git_commit(path: String, message: String) -> Result<(), String> {
  git(&path, &["add", "-A"])?;
  git(&path, &["commit", "-m", &message]).map(|_| ())
}

#[tauri::command]
pub async fn git_push(path: String) -> Result<(), String> {
  // Upstream уже есть — git сам знает, куда пушить
  if ahead(&path).is_some() {
    return git(&path, &["push"]).map(|_| ());
  }
  let remote = push_remote(&path).ok_or("У репозитория нет remote — добавьте его: git remote add origin <url>")?;
  git(&path, &["push", "-u", &remote, "HEAD"]).map(|_| ())
}

/// Без force git откажет, если в worktree есть незакоммиченные изменения — это защита работы агента
#[tauri::command]
pub async fn git_worktree_remove(repo: String, path: String, force: bool) -> Result<(), String> {
  let mut args = vec!["worktree", "remove"];
  if force {
    args.push("--force");
  }
  args.push(&path);
  git(&repo, &args).map(|_| ())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clone_reuses_same_repo_and_rejects_other() {
    let tmp = std::env::temp_dir().join(format!("diogen-clone-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let url = tmp.join("origin.git").to_string_lossy().into_owned();
    git(&tmp.to_string_lossy(), &["init", "--bare", "-q", &url]).unwrap();
    let dir = tmp.join("clones").to_string_lossy().into_owned();
    let clone = |url: &str| tauri::async_runtime::block_on(git_clone(url.into(), dir.clone(), "proj".into()));

    let path = clone(&url).unwrap();
    assert!(Path::new(&path).join(".git").exists());
    assert_eq!(clone(&url).unwrap(), path, "тот же репозиторий — переиспользуем");
    assert!(clone("/elsewhere/other.git").is_err(), "папка занята другим репозиторием");
    std::fs::remove_dir_all(&tmp).ok();
  }

  #[test]
  fn parses_porcelain() {
    let out = "worktree /repo\nHEAD 1111\nbranch refs/heads/main\n\n\
               worktree /wt/feat x\nHEAD 2222\nbranch refs/heads/feat/x\n\n\
               worktree /wt/detached\nHEAD 3333\ndetached\n\n\
               worktree /bare\nbare\n";
    assert_eq!(
      parse_worktrees(out),
      vec![
        Worktree { path: "/repo".into(), branch: Some("main".into()) },
        Worktree { path: "/wt/feat x".into(), branch: Some("feat/x".into()) },
        Worktree { path: "/wt/detached".into(), branch: None },
      ]
    );
  }
}
