use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Статус агента доезжает до вкладки невидимой OSC-последовательностью в его терминале —
/// xterm её перехватывает. Ни сервера, ни портов, ни id панелей.
const STATUS_SH: &str = r#"#!/bin/sh
# Diogen: статус агента ($1 = working|waiting|done|exited) для вкладки, где он запущен
if [ -t 1 ]; then
  # Вызов из shell после выхода агента: stdout и есть терминал
  printf '\033]777;diogen;%s\007' "$1"
else
  # Хук Claude Code: у хуков нет /dev/tty, последовательность выводит сам Claude из terminalSequence.
  # Строго одна строка JSON — иначе UserPromptSubmit добавит вывод в контекст модели
  printf '{"terminalSequence":"\\u001b]777;diogen;%s\\u0007"}\n' "$1"
  # notify Codex: stdout в /dev/null, а управляющий терминал доступен
  { printf '\033]777;diogen;%s\007' "$1" > /dev/tty; } 2>/dev/null
fi
exit 0
"#;

/// Stop не срабатывает при прерывании по Esc — это ловит фронтенд.
/// Notification только про ожидание: idle_prompt через минуту после Stop дублировал бы «закончил»
const CLAUDE_HOOKS: &str = r#"{
  "hooks": {
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "\"$DIOGEN_DIR/status.sh\" working" }] }],
    "PostToolUse": [{ "hooks": [{ "type": "command", "command": "\"$DIOGEN_DIR/status.sh\" working" }] }],
    "PermissionRequest": [{ "hooks": [{ "type": "command", "command": "\"$DIOGEN_DIR/status.sh\" waiting" }] }],
    "Notification": [{ "matcher": "permission_prompt|elicitation_dialog", "hooks": [{ "type": "command", "command": "\"$DIOGEN_DIR/status.sh\" waiting" }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "\"$DIOGEN_DIR/status.sh\" done" }] }]
  }
}
"#;

pub fn dir() -> PathBuf {
  PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".diogen")
}

/// Перезаписываем при каждом старте: файлы всегда соответствуют версии приложения
pub fn install() -> std::io::Result<()> {
  let dir = dir();
  fs::create_dir_all(&dir)?;
  let status = dir.join("status.sh");
  fs::write(&status, STATUS_SH)?;
  fs::set_permissions(&status, fs::Permissions::from_mode(0o755))?;
  fs::write(dir.join("claude-hooks.json"), CLAUDE_HOOKS)
}

#[cfg(test)]
mod tests {
  #[test]
  fn claude_hooks_is_valid_json() {
    let v: serde_json::Value = serde_json::from_str(super::CLAUDE_HOOKS).unwrap();
    assert!(v["hooks"]["Stop"].is_array());
  }
}
