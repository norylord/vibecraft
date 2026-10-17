import type { Component } from 'vue'
import { GitMergeIcon, GitPullRequestIcon, TicketIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'

export type IntegrationId = 'youtrack' | 'gitlab' | 'github'

export interface IntegrationDef {
  id: IntegrationId
  name: string
  icon: Component
  urlPlaceholder: string
  tokenHint: string
  // Запрос «кто я» — проверка подключения
  mePath: string
  meName: (me: any) => string
  // Несекретные параметры — в ~/.vibecraft/state.json, токен — в credentials.json
  options?: { key: string, label: string, default: string, placeholder?: string }[]
  // false — своей страницы и ярлыка в сайдбаре нет: PR и issues GitHub — на странице проекта
  page?: false
  // Токен можно не вводить: GitHub без него берёт токен gh CLI (для Enterprise — по сохранённому адресу)
  tokenOptional?: true
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
    options: [
      { key: 'query', label: 'Запрос для списка задач', default: 'for: me #Unresolved sort by: updated desc' },
      { key: 'reviewState', label: 'Статус задачи после создания MR', default: '', placeholder: 'Не менять, например In Review' },
      { key: 'doneState', label: 'Статус задачи после слияния MR', default: '', placeholder: 'Не менять, например Fixed' },
    ],
  },
  {
    id: 'gitlab',
    name: 'GitLab',
    icon: GitMergeIcon,
    urlPlaceholder: 'https://gitlab.com',
    tokenHint: 'Preferences → Access tokens, scope: api',
    mePath: '/api/v4/user',
    meName: me => me.name || me.username,
    options: [{ key: 'cloneDir', label: 'Папка для клонирования репозиториев', default: '', placeholder: 'Спросим при первом клонировании' }],
  },
  {
    id: 'github',
    name: 'GitHub',
    icon: GitPullRequestIcon,
    urlPlaceholder: 'https://github.com',
    tokenHint: 'Не нужен, если выполнен gh auth login — возьмём токен GitHub CLI. Иначе: Settings → Developer settings → Personal access tokens, scope: repo',
    mePath: '/user',
    meName: me => me.name || me.login,
    page: false,
    tokenOptional: true,
  },
]

// Токены живут в ~/.vibecraft/credentials.json на стороне Rust — сюда приходит только адрес и факт наличия токена
const stored = ref<Partial<Record<IntegrationId, { url: string, hasToken: boolean } | null>>>({})
const options = persisted<Partial<Record<IntegrationId, Record<string, string>>>>('integration-options', {})

async function reload(id: IntegrationId) {
  stored.value[id] = await invoke('integration_get', { id })
}
INTEGRATIONS.forEach(i => reload(i.id))

// Что показывает основная область; workspace при этом не выгружается — терминалы живут.
// repo — страница активного проекта: коммиты, PR/MR и issues
export const useView = () => useState<'workspace' | 'settings' | 'repo' | IntegrationId>('view', () => 'workspace')

export function useIntegrations() {
  const configured = computed(() => INTEGRATIONS.filter(i => stored.value[i.id]?.url && (stored.value[i.id]?.hasToken || i.tokenOptional)))
  // Ярлыки в сайдбаре — только у заполненных интеграций со своей страницей
  const pages = computed(() => configured.value.filter(i => i.page !== false))

  function option(id: IntegrationId, key: string) {
    const def = INTEGRATIONS.find(i => i.id === id)?.options?.find(o => o.key === key)
    return options.value[id]?.[key] || def?.default || ''
  }

  function setOption(id: IntegrationId, key: string, value: string) {
    options.value[id] = { ...options.value[id], [key]: value }
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

  return { stored, configured, pages, option, setOption, save, remove, api, whoami }
}
