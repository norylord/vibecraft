mod files;
mod git;
mod hooks;
mod integrations;
mod pty;
mod ptyhost;
mod state;
mod usage;

/// Открыть путь в приложении (`open -a WebStorm`) или, без app, в Finder
#[tauri::command]
async fn open_path(path: String, app: Option<String>) -> Result<(), String> {
  let mut cmd = std::process::Command::new("open");
  if let Some(app) = &app {
    cmd.args(["-a", app]);
  }
  let out = cmd.arg(&path).output().map_err(|e| e.to_string())?;
  if out.status.success() {
    Ok(())
  } else {
    Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
  }
}

/// Какие из программ есть в PATH пользователя (как в его терминале) — для онбординга агентов
#[tauri::command]
async fn installed(programs: Vec<String>) -> Vec<bool> {
  let path = git::user_path();
  programs.iter().map(|p| std::env::split_paths(path).any(|dir| dir.join(p).is_file())).collect()
}

/// Перезапуск после установки обновления — хост терминалов живёт отдельно, агенты не прерываются
#[tauri::command]
fn restart(app: tauri::AppHandle) {
  app.restart();
}

/// Запуск в роли хоста терминалов (`vibecraft --pty-host`) вместо окна приложения
pub fn pty_host() -> bool {
  if std::env::args().nth(1).as_deref() != Some(ptyhost::ARG) {
    return false;
  }
  let _ = ptyhost::run();
  true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_dialog::init())
    .manage(pty::Ptys::default())
    .invoke_handler(tauri::generate_handler![
      pty::pty_open,
      pty::pty_write,
      pty::pty_resize,
      pty::pty_kill,
      git::git_root,
      git::git_worktrees,
      git::git_branches,
      git::git_fetch,
      git::git_worktree_add,
      git::git_worktree_remove,
      git::git_diff,
      git::git_status,
      git::git_commit,
      git::git_push,
      git::git_remote_url,
      git::git_log,
      git::git_clone,
      open_path,
      installed,
      restart,
      integrations::integration_get,
      integrations::integration_save,
      integrations::integration_delete,
      integrations::api_request,
      hooks::save_prompt,
      files::fs_list,
      files::fs_read,
      files::fs_write,
      files::fs_stat,
      files::fs_head,
      state::state_load,
      state::state_save,
      usage::usage_claude,
      usage::usage_codex,
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      // Без хуков агенты работают, просто без статусов — не роняем приложение
      if let Err(e) = hooks::install() {
        log::error!("не удалось установить хуки агентов: {e}");
      }
      // PATH из login-shell считаем заранее, чтобы первый git-запрос не ждал
      std::thread::spawn(|| git::user_path());
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
