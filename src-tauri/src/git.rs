use std::path::Path;
use std::process::Command;

use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
pub struct Worktree {
  path: String,
  /// None — detached HEAD
  branch: Option<String>,
}

fn git(dir: &str, args: &[&str]) -> Result<String, String> {
  let out = Command::new("git")
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
    let o = Command::new("git")
      .args(["-C", &path, Q[0], Q[1], "diff", "--no-index", "--", "/dev/null", file])
      .output()
      .map_err(|e| e.to_string())?;
    out.push_str(&String::from_utf8_lossy(&o.stdout));
  }
  Ok(out)
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
