mod git;
mod hooks;
mod pty;

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
