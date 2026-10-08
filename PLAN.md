# Diogen ADE — план

Десктопный оркестратор CLI-агентов (Claude Code / Codex / Gemini): задача → изолированный git worktree → PTY с агентом → diff → MR в GitLab ↔ задача в YouTrack.

## Стек

| Слой | Выбор | Почему |
|---|---|---|
| Оболочка | Tauri 2 | Лёгкий бинарник, Rust для PTY/процессов |
| UI | Nuxt 4 (`ssr: false`) + shadcn-vue + Tailwind v4 | Основной стек; command palette, resizable, тёмная тема из коробки |
| Терминал | `@xterm/xterm` + `fit`, `webgl` addons | Стандарт, WebGL держит несколько агентов |
| PTY | `portable-pty` (crate), стрим через `tauri::ipc::Channel` | Канал быстрее events |
| Git | `git` CLI из Rust | Твои конфиги/хуки работают, `git worktree list --porcelain` — источник правды |
| Diff | `@codemirror/merge` | Легче Monaco |
| HTTP к API | `@tauri-apps/plugin-http` | Обходит CORS self-hosted GitLab/YouTrack |
| Секреты | `keyring` crate (macOS Keychain) | 2 команды get/set |
| Состояние | `tauri-plugin-store` (JSON) | SQLite не нужен на старте |

## Архитектура

```
Nuxt SPA ──invoke/Channel──► Rust core
 ├ Sidebar: проекты → worktree      ├ pty.rs     spawn/write/resize/kill
 ├ Center: xterm-панели агентов     ├ git.rs     worktree add/list/remove, diff, commit, push
 ├ Right: diff / задача / MR        ├ secrets.rs keyring get/set
 └ composables: usePty, useWorktrees,└ events.rs  локальный HTTP для хуков агентов
   useYouTrack, useGitLab
```

**Статус агента** (работает / ждёт ввода / завершил): в env PTY передаём `DIOGEN_PORT` + `DIOGEN_PANE`; хуки Claude Code (`Stop`, `Notification`) → `curl` на `127.0.0.1:$DIOGEN_PORT`. Codex — `notify`. Фолбэк для прочих — тишина в PTY N секунд.

## Интеграции

- **YouTrack** — REST `/api/issues` + permanent token. Список «мои задачи» → **Start** → worktree `ABC-123-slug` + агент с промптом из summary/description → по созданию MR комментарий и смена статуса.
- **GitLab** — REST v4 + PAT. Создать MR из ветки worktree, статус пайплайна, ссылка в правой панели.
- **Figma** — без своего кода: Figma MCP уже есть у агентов. ADE хранит MCP-пресет (`.mcp.json` в worktree) и прокидывает ссылку из задачи в промпт.

## Этапы (≈4 недели)

1. **Каркас (2–3 дня)** — Tauri + Nuxt 4 SPA + shadcn-vue, 3-панельный layout, overlay-titlebar + vibrancy (macOS), command palette `⌘K`, горячие клавиши.
2. **Ядро (нед. 1)** — PTY ↔ xterm, CRUD worktree, пресеты агентов (команда + args + env), запуск агента в worktree, персист проектов.
3. **Контроль (нед. 2)** — статусы через хуки, системные уведомления (`tauri-plugin-notification`), diff-view, commit/push, «Открыть в WebStorm».
4. **Интеграции (нед. 3)** — YouTrack → worktree+агент, GitLab MR/пайплайны, MCP-пресеты (Figma).
5. **Полировка (нед. 4)** — темы, анимации, онбординг токенов, сборка `.dmg`.

## Риски

- Флуд вывода агентов → батчить PTY-вывод в Rust (~16 мс).
- ACL Tauri 2: каждый плагин/команду явно разрешать в `capabilities/default.json`, иначе тихие отказы.
- Агенты умирают при закрытии ADE → позже запуск внутри `tmux`.

## Вне MVP

Свой редактор (есть WebStorm), свой агент, SQLite, синхронизация — добавлять, когда упрёмся.
