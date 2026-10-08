export interface TerminalTab { key: string, title: string, cwd?: string }

// SPA (ssr: false) — модульное состояние общее на всё приложение
const terminals = ref<TerminalTab[]>([])
const activeTerminal = ref<string>()

export function useTerminals() {
  function openTerminal(cwd?: string) {
    const tab = { key: crypto.randomUUID(), title: 'Терминал', cwd }
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

  return { terminals, activeTerminal, openTerminal, closeTerminal, closeTerminalsIn }
}
