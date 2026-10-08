import type { AgentPreset } from './useAgents'
import { invoke } from '@tauri-apps/api/core'

export interface Issue {
  id: string
  summary: string
  description: string
  updated: number
  project: string
  state?: string
  priority?: string
}

const FIELDS = 'idReadable,summary,description,updated,project(shortName),customFields(name,value(name))'
// Названия полей настраиваются в каждом инстансе — берём распространённые
const STATE = ['State', 'Состояние', 'Статус']
const PRIORITY = ['Priority', 'Приоритет']

// worktree → задача: для вкладки «Задача» и комментария при создании MR
const links = persisted<Record<string, string>>('worktree-issues', {})
// YouTrack-проект → локальный репозиторий: подставляем прошлый выбор
const repoFor = persisted<Record<string, string>>('youtrack-repos', {})

function toIssue(raw: any): Issue {
  const field = (names: string[]) => raw.customFields?.find((f: any) => names.includes(f.name))?.value?.name
  return {
    id: raw.idReadable,
    summary: raw.summary,
    description: raw.description ?? '',
    updated: raw.updated,
    project: raw.project?.shortName ?? '',
    state: field(STATE),
    priority: field(PRIORITY),
  }
}

export function useYouTrack() {
  const { api, option, stored } = useIntegrations()
  const { worktrees, activeWorktree, refresh, createWorktree } = useProjects()
  const { openTerminal } = useTerminals()

  async function issues() {
    const query = encodeURIComponent(option('youtrack', 'query'))
    const raw = await api<any[]>('youtrack', `/api/issues?query=${query}&fields=${FIELDS}&$top=100`)
    return raw.map(toIssue)
  }

  const issue = async (id: string) => toIssue(await api('youtrack', `/api/issues/${id}?fields=${FIELDS}`))

  const comment = (id: string, text: string) => api('youtrack', `/api/issues/${id}/comments?fields=id`, 'POST', { text })

  const issueUrl = (id: string) => `${stored.value.youtrack?.url}/issue/${id}`

  // Worktree под задачу (существующий для этой ветки переиспользуем) + агент с промптом из задачи.
  // Возвращает 'clipboard', если агент не принимает промпт аргументом и его надо вставить руками
  async function start(issue: Issue, repo: string, branch: string, agent: AgentPreset) {
    repoFor.value[issue.project] = repo
    await refresh(repo)
    const path = worktrees.value[repo]?.find(w => w.branch === branch)?.path ?? await createWorktree(repo, branch)
    activeWorktree.value = path
    links.value[path] = issue.id

    const prompt = `Задача ${issue.id}: ${issue.summary}\n${issueUrl(issue.id)}\n\n${issue.description}`.trim()
    await invoke('save_prompt', { name: issue.id, text: prompt })
    if (acceptsPrompt(agent.command)) {
      openTerminal(path, agent, issue.id)
      return
    }
    openTerminal(path, agent)
    await navigator.clipboard.writeText(prompt)
    return 'clipboard' as const
  }

  return { issues, issue, comment, issueUrl, start, links, repoFor }
}
