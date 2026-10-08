import type { Component } from 'vue'
import { GitMergeIcon, TicketIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { useLocalStorage } from '@vueuse/core'

export type IntegrationId = 'youtrack' | 'gitlab'

export interface IntegrationDef {
  id: IntegrationId
  name: string
  icon: Component
  urlPlaceholder: string
  tokenHint: string
  // Запрос «кто я» — проверка подключения
  mePath: string
  meName: (me: any) => string
  // Несекретные параметры — в localStorage, не в Keychain
  options?: { key: string, label: string, default: string }[]
}

export const INTEGRATIONS: IntegrationDef[] = [
  {
    id: 'youtrack',
    name: 'YouTrack',
    icon: TicketIcon,
    urlPlaceholder: 'https://company.youtrack.cloud',
    tokenHint: 'Профиль → Account Security → New token',
    mePath: '/api/users/me?fields=login,fullName',
    meName: me => me.fullName || me.login,
    options: [{ key: 'query', label: 'Запрос для списка задач', default: 'for: me #Unresolved' }],
  },
  {
    id: 'gitlab',
    name: 'GitLab',
    icon: GitMergeIcon,
    urlPlaceholder: 'https://gitlab.com',
    tokenHint: 'Preferences → Access tokens, scope: api',
    mePath: '/api/v4/user',
    meName: me => me.name || me.username,
  },
]

// Токены живут в Keychain на стороне Rust — сюда приходит только адрес и факт наличия токена
const stored = ref<Partial<Record<IntegrationId, { url: string, hasToken: boolean } | null>>>({})
const options = useLocalStorage<Partial<Record<IntegrationId, Record<string, string>>>>('diogen:integration-options', {})

async function reload(id: IntegrationId) {
  stored.value[id] = await invoke('integration_get', { id })
}
INTEGRATIONS.forEach(i => reload(i.id))

// Что показывает основная область; workspace при этом не выгружается — терминалы живут
export const useView = () => useState<'workspace' | 'settings' | IntegrationId>('view', () => 'workspace')

export function useIntegrations() {
  // Ярлыки в сайдбаре — только у заполненных интеграций
  const configured = computed(() => INTEGRATIONS.filter(i => stored.value[i.id]?.url && stored.value[i.id]?.hasToken))

  function option(id: IntegrationId, key: string) {
    const def = INTEGRATIONS.find(i => i.id === id)?.options?.find(o => o.key === key)
    return options.value[id]?.[key] || def?.default || ''
  }

  // token: undefined — оставить сохранённый
  async function save(id: IntegrationId, url: string, token?: string, values?: Record<string, string>) {
    await invoke('integration_save', { id, url, token: token || undefined })
    if (values) options.value[id] = values
    await reload(id)
  }

  async function remove(id: IntegrationId) {
    await invoke('integration_delete', { id })
    await reload(id)
  }

  const api = <T = any>(id: IntegrationId, path: string, method?: string, body?: unknown) =>
    invoke<T>('api_request', { id, path, method, body })

  async function whoami(id: IntegrationId) {
    const def = INTEGRATIONS.find(i => i.id === id)!
    return def.meName(await api(id, def.mePath))
  }

  return { stored, configured, option, save, remove, api, whoami }
}
