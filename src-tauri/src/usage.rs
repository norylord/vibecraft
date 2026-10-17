use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

/// Лимиты подписки Claude — тот же запрос, что делает /usage в Claude Code.
/// OAuth-токен Claude Code хранит в Keychain через `security`, поэтому он же читает его без запроса пароля.
/// Токен не обновляем: refresh-токен одноразовый, это забота самого Claude Code
#[tauri::command]
pub async fn usage_claude() -> Result<Value, String> {
  let out = Command::new("security")
    .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
    .output()
    .map_err(|e| e.to_string())?;
  if !out.status.success() {
    return Err("Claude Code не авторизован по подписке".into());
  }
  let creds: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
  let token = creds["claudeAiOauth"]["accessToken"].as_str().ok_or("Claude Code не авторизован по подписке")?;
  let res = reqwest::Client::new()
    .get("https://api.anthropic.com/api/oauth/usage")
    .bearer_auth(token)
    .header("anthropic-beta", "oauth-2025-04-20")
    .timeout(Duration::from_secs(20))
    .send()
    .await
    .map_err(|e| e.to_string())?;
  match res.status().as_u16() {
    200 => res.json().await.map_err(|e| e.to_string()),
    401 => Err("Токен истёк — запустите Claude Code, он обновит его".into()),
    status => Err(format!("api.anthropic.com: {status}")),
  }
}

/// Лимиты Codex через его app-server (JSON-RPC построчно в stdio): сам Codex знает, где его токен
#[tauri::command]
pub async fn usage_codex() -> Result<Value, String> {
  tauri::async_runtime::spawn_blocking(codex_limits).await.map_err(|e| e.to_string())?
}

fn codex_limits() -> Result<Value, String> {
  let mut child = Command::new("codex")
    .arg("app-server")
    .env("PATH", crate::git::user_path())
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .map_err(|e| format!("codex: {e}"))?;
  // stdin держим открытым до ответа: на EOF app-server завершается
  let mut stdin = child.stdin.take().unwrap();
  let requests = [
    json!({ "method": "initialize", "id": 0, "params": { "clientInfo": { "name": "vibecraft", "title": "Vibecraft", "version": env!("CARGO_PKG_VERSION") }, "capabilities": null } }),
    json!({ "method": "initialized" }),
    json!({ "method": "account/rateLimits/read", "id": 1, "params": { "excludeResetCreditDetails": true } }),
  ];
  for r in requests {
    writeln!(stdin, "{r}").map_err(|e| e.to_string())?;
  }
  let stdout = child.stdout.take().unwrap();
  let (tx, rx) = mpsc::channel();
  std::thread::spawn(move || {
    let reply = BufReader::new(stdout)
      .lines()
      .map_while(Result::ok)
      .filter_map(|l| serde_json::from_str::<Value>(&l).ok())
      .find(|m| m["id"] == 1);
    let _ = tx.send(reply);
  });
  // Зависший codex не должен копиться процессами: ждём ограниченно и убиваем в любом случае
  let reply = rx.recv_timeout(Duration::from_secs(20)).ok().flatten();
  let _ = child.kill();
  let _ = child.wait();
  drop(stdin);
  let reply = reply.ok_or("codex app-server не ответил")?;
  if let Some(e) = reply.get("error") {
    return Err(e["message"].as_str().unwrap_or("ошибка codex app-server").to_owned());
  }
  Ok(reply["result"]["rateLimits"].clone())
}
