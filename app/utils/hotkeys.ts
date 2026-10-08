export const ACTIONS = [
  { id: 'palette', label: 'Палитра команд', default: 'Meta+KeyK' },
  { id: 'sidebar', label: 'Показать / скрыть сайдбар', default: 'Meta+KeyB' },
  { id: 'panel', label: 'Показать / скрыть правую панель', default: 'Meta+KeyJ' },
  { id: 'terminal', label: 'Новый терминал', default: 'Meta+KeyT' },
  { id: 'settings', label: 'Настройки', default: 'Meta+Comma' },
  { id: 'zoomIn', label: 'Увеличить масштаб', default: 'Meta+Equal' },
  { id: 'zoomOut', label: 'Уменьшить масштаб', default: 'Meta+Minus' },
  { id: 'zoomReset', label: 'Сбросить масштаб', default: 'Meta+Digit0' },
] as const

export type ActionId = typeof ACTIONS[number]['id']

// Сочетания macOS и правки текста: перехват сломал бы систему или копирование в терминале
export const RESERVED = ['Meta+KeyQ', 'Meta+KeyW', 'Meta+KeyH', 'Meta+KeyM', 'Meta+KeyC', 'Meta+KeyV', 'Meta+KeyX', 'Meta+KeyA', 'Meta+KeyZ', 'Meta+Shift+KeyZ']

const MODIFIERS = [['Meta', '⌘'], ['Ctrl', '⌃'], ['Alt', '⌥'], ['Shift', '⇧']] as const
const KEY_LABELS: Record<string, string> = {
  Comma: ',', Period: '.', Slash: '/', Backslash: '\\', Semicolon: ';', Quote: '\'', Backquote: '`',
  BracketLeft: '[', BracketRight: ']', Minus: '−', Equal: '=', Space: 'Space', Enter: '↵', Tab: '⇥',
  ArrowUp: '↑', ArrowDown: '↓', ArrowLeft: '←', ArrowRight: '→', Backspace: '⌫', Escape: 'Esc',
}

/** Сочетание из события: по `code` — не зависит от раскладки. Без модификатора — не хоткей */
export function comboOf(e: KeyboardEvent) {
  if (['Meta', 'Control', 'Alt', 'Shift'].includes(e.key)) return
  const mods = [e.metaKey && 'Meta', e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift'].filter(Boolean)
  if (!mods.length || (mods.length === 1 && e.shiftKey)) return
  return [...mods, e.code].join('+')
}

/** `Meta+Shift+KeyK` → `⌘⇧K` */
export function formatCombo(combo: string) {
  const parts = combo.split('+')
  const key = parts.at(-1)!
  const mods = MODIFIERS.filter(([m]) => parts.includes(m)).map(([, s]) => s).join('')
  return mods + (KEY_LABELS[key] ?? key.replace(/^Key|^Digit/, ''))
}
