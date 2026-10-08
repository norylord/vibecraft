import type { ActionId } from '../utils/hotkeys'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useMediaQuery } from '@vueuse/core'
import { ACTIONS, formatCombo } from '../utils/hotkeys'

export type Theme = 'dark' | 'light' | 'system'

const settings = persisted('settings', {
  theme: 'dark' as Theme,
  zoom: 1,
  terminalFontSize: 13,
  editor: 'WebStorm',
  notifications: true,
  collapsePanelOnStart: true,
  // Только переопределённые сочетания; остальные — default из ACTIONS
  hotkeys: {} as Partial<Record<ActionId, string>>,
})

export type SettingsTab = 'general' | 'appearance' | 'hotkeys' | 'agents' | 'integrations'

const systemDark = useMediaQuery('(prefers-color-scheme: dark)')
const isDark = computed(() => settings.value.theme === 'system' ? systemDark.value : settings.value.theme === 'dark')

export const ZOOM_MIN = 0.5
export const ZOOM_MAX = 2

export function useSettings() {
  const view = useView()
  const tab = useState<SettingsTab>('settings-tab', () => 'general')

  const binding = (id: ActionId) => settings.value.hotkeys[id] ?? ACTIONS.find(a => a.id === id)!.default
  const shortcut = (id: ActionId) => formatCombo(binding(id))

  function openSettings(section: SettingsTab = tab.value) {
    tab.value = section
    view.value = 'settings'
  }

  // ±10%: шаг, привычный по браузерам
  function zoomBy(delta: number) {
    const next = Math.round((settings.value.zoom + delta) * 10) / 10
    settings.value.zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, next))
  }

  return { settings, isDark, tab, binding, shortcut, openSettings, zoomBy }
}

/** Тема — классом .dark и внешним видом окна macOS (размытие под сайдбаром), масштаб — зумом webview. Вызывать один раз в app.vue */
export function applyAppearance() {
  watchEffect(() => {
    document.documentElement.classList.toggle('dark', isDark.value)
    const theme = settings.value.theme
    getCurrentWindow().setTheme(theme === 'system' ? null : theme)
  })
  watchEffect(() => getCurrentWebview().setZoom(settings.value.zoom))
}
