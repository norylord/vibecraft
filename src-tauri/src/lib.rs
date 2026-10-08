mod git;
mod hooks;
mod integrations;
mod pty;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_dialog::init())
    .manage(pty::Ptys::default())
    .invoke_handler(tauri::generate_handler![
      pty::pty_spawn,
      pty::pty_write,
      pty::pty_resize,
      pty::pty_kill,
      git::git_root,
      git::git_worktrees,
      git::git_worktree_add,
      git::git_worktree_remove,
      git::git_diff,
      git::git_status,
      git::git_commit,
      git::git_push,
      open_path,
      integrations::integration_get,
      integrations::integration_save,
      integrations::integration_delete,
      integrations::api_request,
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
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
