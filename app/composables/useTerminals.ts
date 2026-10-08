import type { AgentPreset } from './useAgents'

export type AgentStatus = 'working' | 'waiting' | 'done'

// command — что напечатать в shell после старта (агент); без неё — просто терминал.
// tracked — агент шлёт статусы хуками; workingOnEnter — у него нет хука «начал работу» (Codex)
export interface TerminalTab {
  key: string
  title: string
  cwd?: string
  command?: string
  status?: AgentStatus
  tracked?: boolean
  workingOnEnter?: boolean
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

export function useTerminals() {
  // prompt — имя файла из save_prompt (без .md)
  function openTerminal(cwd?: string, agent?: AgentPreset, prompt?: string) {
    const tab: TerminalTab = { key: crypto.randomUUID(), title: agent?.name ?? 'Терминал', cwd, ...(agent && withHooks(agent.command, prompt)) }
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

  return { terminals, activeTerminal, openTerminal, closeTerminal, closeTerminalsIn, statusIn }
}
