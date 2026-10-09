import type { AgentPreset } from './useAgents'
import { invoke } from '@tauri-apps/api/core'

export type AgentStatus = 'working' | 'waiting' | 'done'

// command — что напечатать в shell после старта (агент); без неё — просто терминал.
// tracked — агент шлёт статусы хуками; workingOnEnter — у него нет хука «начал работу» (Codex).
// name — исходный заголовок (title меняет сам терминал); exited — агент вышел, остался shell;
// restored — вкладка из прошлого запуска: переподключаемся к сессии в хосте, ничего не запуская
export interface TerminalTab {
  key: string
  title: string
  name: string
  cwd?: string
  command?: string
  status?: AgentStatus
  tracked?: boolean
  workingOnEnter?: boolean
  exited?: boolean
  restored?: boolean
}

// Программа агента без префикса из переменных: `FOO=1 claude --model x` → claude
export const agentProgram = (command: string) => command.trim().split(/\s+/).find(w => !w.includes('='))

// Начальный промпт позиционным аргументом понимают claude и codex; остальным его вставляют вручную
export const acceptsPrompt = (command: string) => ['claude', 'codex'].includes(agentProgram(command) ?? '')

// Хуки статусов подмешиваются флагами запуска — конфиги пользователя не трогаем.
// Промпт — файл из save_prompt: "$(cat ...)" избавляет от экранирования текста задачи в shell.
// Хвост `status.sh exited` снимает статус, когда агент вышел и остался shell
function withHooks(command: string, prompt?: string) {
  const p = agentProgram(command)
  const flags = p === 'claude'
    ? ' --settings "$DIOGEN_DIR/claude-hooks.json"'
    : p === 'codex'
      ? ' -c "notify=[\\"$DIOGEN_DIR/status.sh\\",\\"done\\"]"'
      : undefined
  if (!flags) return { command }
  const promptArg = prompt ? ` "$(cat "$DIOGEN_DIR/prompts/${prompt}.md")"` : ''
  return { command: `${command}${flags}${promptArg}; "$DIOGEN_DIR/status.sh" exited`, tracked: true, workingOnEnter: p === 'codex' }
}

// Сводный статус worktree для сайдбара: важнее то, что требует внимания
const STATUS_PRIORITY: AgentStatus[] = ['waiting', 'working', 'done']

// SPA (ssr: false) — модульное состояние общее на всё приложение
const terminals = ref<TerminalTab[]>([])
const activeTerminal = ref<string>()

// Вкладки переживают перезапуск вместе с сессиями в хосте терминалов (src-tauri/src/ptyhost.rs).
// Сохраняем только нужное для переподключения: title терминал меняет на лету — state.json писался бы постоянно.
// У dev-сборки свой список, как и свой хост
const saved = persisted<Omit<TerminalTab, 'title'>[]>(import.meta.dev ? 'terminals-dev' : 'terminals', [])
watch(
  () => terminals.value.map(({ key, name, cwd, command, status, tracked, workingOnEnter }) => ({ key, name, cwd, command, status, tracked, workingOnEnter })),
  tabs => (saved.value = tabs),
)
let restored = false

export function useTerminals() {
  // persisted загружается плагином до монтирования — к первому вызову из компонента данные уже на месте
  if (!restored) {
    restored = true
    terminals.value = saved.value.map(t => ({ ...t, title: t.name, restored: true }))
  }

  // prompt — имя файла из save_prompt (без .md)
  function openTerminal(cwd?: string, agent?: AgentPreset, prompt?: string) {
    const name = agent?.name ?? 'Терминал'
    const tab: TerminalTab = { key: crypto.randomUUID(), title: name, name, cwd, ...(agent && withHooks(agent.command, prompt)) }
    terminals.value.push(tab)
    activeTerminal.value = tab.key
  }

  // Активную вкладку после закрытия выбирает Workspace
  function closeTerminal(key: string) {
    terminals.value = terminals.value.filter(t => t.key !== key)
  }

  function closeTerminalsIn(cwd: string) {
    terminals.value = terminals.value.filter(t => t.cwd !== cwd)
  }

  const statusIn = (cwd: string) =>
    STATUS_PRIORITY.find(s => terminals.value.some(t => t.cwd === cwd && t.status === s))

  const { runnable } = useAgents()

  // Текст агенту worktree: запущенному — вставкой и Enter, иначе — новый агент с этим промптом.
  // Вставляем только в живого агента: в shell Enter выполнил бы текст как команды.
  // name — имя файла промпта для save_prompt. false — нет агента, который принимает промпт
  async function sendToAgent(cwd: string, name: string, text: string) {
    const tab = terminals.value.findLast(t => t.cwd === cwd && t.tracked && !t.exited)
    if (tab) {
      // Bracketed paste: многострочный текст агент примет целиком, а не построчными отправками
      await invoke('pty_write', { key: tab.key, data: `\x1B[200~${text}\x1B[201~` })
      // Enter отдельной записью: в одной с текстом агент счёл бы его частью вставки
      setTimeout(() => invoke('pty_write', { key: tab.key, data: '\r' }), 200)
      activeTerminal.value = tab.key
      return true
    }
    const agent = runnable.value.find(a => acceptsPrompt(a.command))
    if (!agent) return false
    await invoke('save_prompt', { name, text })
    openTerminal(cwd, agent, name)
    return true
  }

  return { terminals, activeTerminal, openTerminal, closeTerminal, closeTerminalsIn, statusIn, sendToAgent }
}
