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

// Хуки статусов подмешиваются флагами запуска — конфиги пользователя не трогаем.
// Хвост `status.sh exited` снимает статус, когда агент вышел и остался shell
function withHooks(command: string) {
  const program = command.trim().split(/\s+/).find(w => !w.includes('='))
  const flags = program === 'claude'
    ? ' --settings "$DIOGEN_DIR/claude-hooks.json"'
    : program === 'codex'
      ? ' -c "notify=[\\"$DIOGEN_DIR/status.sh\\",\\"done\\"]"'
      : undefined
  if (!flags) return { command }
  return { command: `${command}${flags}; "$DIOGEN_DIR/status.sh" exited`, tracked: true, workingOnEnter: program === 'codex' }
}

// Сводный статус worktree для сайдбара: важнее то, что требует внимания
const STATUS_PRIORITY: AgentStatus[] = ['waiting', 'working', 'done']

// SPA (ssr: false) — модульное состояние общее на всё приложение
const terminals = ref<TerminalTab[]>([])
const activeTerminal = ref<string>()

export function useTerminals() {
  function openTerminal(cwd?: string, agent?: AgentPreset) {
    const tab: TerminalTab = { key: crypto.randomUUID(), title: agent?.name ?? 'Терминал', cwd, ...(agent && withHooks(agent.command)) }
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
