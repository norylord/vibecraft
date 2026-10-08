// Команда печатается в login-shell worktree: флаги и env пишутся прямо в ней (`FOO=1 claude --model opus`)
export interface AgentPreset { name: string, command: string }

const agents = persisted<AgentPreset[]>('agents', [
  { name: 'Claude Code', command: 'claude' },
  { name: 'Codex', command: 'codex' },
  { name: 'Gemini', command: 'gemini' },
])
// Недозаполненные строки из диалога настройки в меню не показываем
const runnable = computed(() => agents.value.filter(a => a.name.trim() && a.command.trim()))

export const useAgents = () => ({ agents, runnable })
