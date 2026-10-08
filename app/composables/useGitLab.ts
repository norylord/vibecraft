import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

export interface MergeRequest {
  iid: number
  title: string
  url: string
  state: string
  draft: boolean
  source: string
  target: string
  project: string
  updated: string
  pipeline?: { status: string, url: string }
}

function toMR(raw: any): MergeRequest {
  return {
    iid: raw.iid,
    title: raw.title,
    url: raw.web_url,
    state: raw.state,
    draft: raw.draft ?? raw.work_in_progress ?? false,
    source: raw.source_branch,
    target: raw.target_branch,
    project: raw.references?.full?.replace(/!\d+$/, '') ?? '',
    updated: raw.updated_at,
    pipeline: raw.head_pipeline && { status: raw.head_pipeline.status, url: raw.head_pipeline.web_url },
  }
}

export interface GitLabProject {
  path: string
  name: string
  description: string
  sshUrl: string
  url: string
  activity: string
}

const enc = encodeURIComponent

export function useGitLab() {
  const { api, stored, option, setOption } = useIntegrations()
  const { addPath } = useProjects()
  const gl = <T = any>(path: string, method?: string, body?: unknown) => api<T>('gitlab', `/api/v4${path}`, method, body)

  // Путь проекта GitLab по remote worktree; undefined — GitLab не настроен или remote не на нём
  async function projectFor(worktree: string) {
    const remote = await invoke<string | null>('git_remote_url', { path: worktree })
    const url = stored.value.gitlab?.url
    return remote && url ? gitlabProject(remote, url) : undefined
  }

  // Открытый MR из ветки — с пайплайном (его отдаёт только запрос одного MR)
  async function mrFor(project: string, branch: string) {
    const [found] = await gl<any[]>(`/projects/${enc(project)}/merge_requests?state=opened&source_branch=${enc(branch)}`)
    return found && toMR(await gl(`/projects/${enc(project)}/merge_requests/${found.iid}`))
  }

  const defaultBranch = async (project: string) => (await gl(`/projects/${enc(project)}`)).default_branch as string

  const createMR = async (project: string, mr: { source: string, target: string, title: string, description: string }) =>
    toMR(await gl(`/projects/${enc(project)}/merge_requests`, 'POST', {
      source_branch: mr.source,
      target_branch: mr.target,
      title: mr.title,
      description: mr.description,
      remove_source_branch: true,
    }))

  // created — мои MR, review — где я ревьюер
  async function myMRs(kind: 'created' | 'review') {
    const query = kind === 'created'
      ? 'scope=created_by_me'
      : `scope=all&reviewer_username=${enc((await gl('/user')).username)}`
    return (await gl<any[]>(`/merge_requests?state=opened&per_page=50&${query}`)).map(toMR)
  }

  // Проекты, где я участник, — последние по активности
  const projects = async (): Promise<GitLabProject[]> =>
    (await gl<any[]>('/projects?membership=true&simple=true&order_by=last_activity_at&per_page=100')).map(p => ({
      path: p.path_with_namespace,
      name: p.path,
      description: p.description ?? '',
      sshUrl: p.ssh_url_to_repo,
      url: p.web_url,
      activity: p.last_activity_at,
    }))

  const cloneDir = computed(() => option('gitlab', 'cloneDir'))
  const clonePath = (p: GitLabProject) => cloneDir.value && `${cloneDir.value.replace(/\/+$/, '')}/${p.name}`

  // Папку спрашиваем один раз, дальше подставляется сама; сменить — здесь же или в настройках
  async function pickCloneDir() {
    const dir = await open({ directory: true, title: 'Куда клонировать репозитории GitLab' })
    if (dir) setOption('gitlab', 'cloneDir', dir)
    return dir ?? undefined
  }

  // Клон по SSH + проект сразу в сайдбаре; уже склонированный — просто добавляем
  async function clone(p: GitLabProject) {
    const dir = cloneDir.value || await pickCloneDir()
    if (!dir) return
    const path = await invoke<string>('git_clone', { url: p.sshUrl, dir, name: p.name })
    return addPath(path)
  }

  return { projectFor, mrFor, defaultBranch, createMR, myMRs, projects, cloneDir, clonePath, pickCloneDir, clone }
}
