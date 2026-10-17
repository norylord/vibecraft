import type { MergeRequest } from './useGitLab'
import type { ForgePage } from '@/utils/forge'
import { invoke } from '@tauri-apps/api/core'

export interface RepoIssue {
  iid: number
  title: string
  url: string
  author: string
  labels: string[]
  updated: string
}

export type RepoTab = 'commits' | 'mrs' | 'issues'

// remote по корню репозитория (worktree делят его с основным); null — remote нет
const remotes = ref<Record<string, string | null>>({})
// Открытые PR/MR и issues по странице репозитория (Forge.url): счётчики футера и списки страницы проекта
const lists = ref<Record<string, { mrs?: ForgePage<MergeRequest>, issues?: ForgePage<RepoIssue>, error?: string }>>({})

// Вкладка страницы проекта — её же подсвечивает футер
export const useRepoTab = () => useState<RepoTab>('repo-tab', () => 'commits')

/** Хостинг активного проекта по его remote и API хостинга: GitHub и GitLab отдают одни и те же методы */
export function useForge() {
  const { active } = useProjects()
  const { stored, configured } = useIntegrations()
  const apis = { github: useGitHub(), gitlab: useGitLab() }

  const repo = computed(() => active.value?.wt.git ? active.value.repo : undefined)
  // undefined — ещё не прочитан
  const remote = computed(() => repo.value ? remotes.value[repo.value] : undefined)
  const forge = computed(() => {
    if (!remote.value) return
    // GitHub без токена в настройках работает через gh CLI, GitLab — только настроенный
    const gitlab = configured.value.some(i => i.id === 'gitlab') ? stored.value.gitlab?.url : undefined
    return forgeOf(remote.value, { github: stored.value.github?.url, gitlab })
  })
  const api = computed(() => forge.value?.api ? apis[forge.value.kind as keyof typeof apis] : undefined)
  const terms = computed(() => forgeTerms(forge.value))
  const current = computed(() => forge.value && lists.value[forge.value.url])

  async function loadRemote() {
    const r = repo.value
    if (r) remotes.value[r] = await invoke<string | null>('git_remote_url', { path: r })
  }

  // Ошибку храним рядом с прошлым списком: при сбое сети счётчики не пропадают
  async function loadLists() {
    const f = forge.value
    const a = api.value
    if (!f || !a) return
    try {
      const [mrs, issues] = await Promise.all([a.openMRs(f.path), a.openIssues(f.path)])
      lists.value[f.url] = { mrs, issues }
    }
    catch (e) {
      lists.value[f.url] = { ...lists.value[f.url], error: String(e) }
    }
  }

  return { repo, remote, forge, api, terms, current, loadRemote, loadLists }
}
