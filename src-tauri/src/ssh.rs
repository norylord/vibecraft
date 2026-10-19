use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::Serialize;

/// SSH_ASKPASS серверов с сохранённым паролем: с SSH_ASKPASS_REQUIRE=force ssh шлёт сюда все вопросы.
/// Пароль — запись $VIBECRAFT_SSH в credentials.json — отдаём только на первый запрос пароля или passphrase:
/// повтор после неверного пароля, yes/no про ключ хоста и коды 2FA спрашиваем в терминале
const ASKPASS_SH: &str = r#"#!/bin/sh
case "$1" in
  *assword*|*assphrase*)
    # ponytail: метка по pid ssh не удаляется — тот же pid позже получит вопрос в терминале вместо пароля
    once="${TMPDIR:-/tmp}/vibecraft-askpass-$PPID"
    if [ ! -e "$once" ] && touch "$once" && /usr/bin/plutil -extract "$VIBECRAFT_SSH.token" raw -o - "$VIBECRAFT_DIR/credentials.json" 2>/dev/null; then
      exit 0
    fi
    hidden=1 ;;
esac
printf '%s' "$1" > /dev/tty
[ "$hidden" ] && stty -echo < /dev/tty
IFS= read -r answer < /dev/tty
[ "$hidden" ] && { stty echo < /dev/tty; echo > /dev/tty; }
printf '%s\n' "$answer"
"#;

/// Как hooks::install — перезаписываем при каждом старте
pub fn install() -> std::io::Result<()> {
  let path = crate::hooks::dir().join("askpass.sh");
  fs::write(&path, ASKPASS_SH)?;
  fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
}

#[derive(Serialize)]
pub struct Local {
  keys: Vec<String>,
  config: String,
}

/// Подсказки для формы сервера: приватные ключи из ~/.ssh (рядом лежит .pub) и текст ~/.ssh/config
#[tauri::command]
pub async fn ssh_local() -> Local {
  let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".ssh");
  let mut keys: Vec<String> = fs::read_dir(&dir)
    .into_iter()
    .flatten()
    .flatten()
    .filter_map(|e| e.file_name().to_str()?.strip_suffix(".pub").map(str::to_owned))
    .filter(|k| dir.join(k).is_file())
    .map(|k| format!("~/.ssh/{k}"))
    .collect();
  keys.sort();
  Local { keys, config: fs::read_to_string(dir.join("config")).unwrap_or_default() }
}

#[cfg(test)]
mod tests {
  use std::fs;

  #[test]
  fn askpass_gives_saved_password_on_first_prompt() {
    let dir = std::env::temp_dir().join(format!("vibecraft-askpass-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("askpass.sh"), super::ASKPASS_SH).unwrap();
    fs::write(dir.join("credentials.json"), r#"{"ssh-1":{"url":"","token":"p@ss \"w\" ü"}}"#).unwrap();
    let out = std::process::Command::new("sh")
      .arg(dir.join("askpass.sh"))
      .arg("user@host's password: ")
      .env("VIBECRAFT_DIR", &dir)
      .env("VIBECRAFT_SSH", "ssh-1")
      .env("TMPDIR", &dir)
      .output()
      .unwrap();
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "p@ss \"w\" ü\n");
  }
}
