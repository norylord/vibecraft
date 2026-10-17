import type { RepoIssue } from './useForge'
import type { MergeRequest } from './useGitLab'

// PR в общем виде MergeRequest — панель PR/MR и страница проекта не различают хостинги
function toMR(raw: any, pipeline?: MergeRequest['pipeline']): MergeRequest {
  return {
    iid: raw.number,
    title: raw.title,
    url: raw.html_url,
    state: raw.merged_at ? 'merged' : raw.state === 'open' ? 'opened' : 'closed',
    draft: raw.draft ?? false,
    source: raw.head?.ref ?? '',
    target: raw.base?.ref ?? '',
    project: raw.base?.repo?.full_name ?? '',
    updated: raw.updated_at,
    author: raw.user?.login,
    pipeline,
  }
}

// repo — `owner/name` (Forge.path): в пути API слэш остаётся как есть
export function useGitHub() {
  const { api } = useIntegrations()
  const gh = <T = any>(path: string, method?: string, body?: unknown) => api<T>('github', path, method, body)

  const checkRuns = async (repo: string, sha: string) =>
    (await gh(`/repos/${repo}/commits/${sha}/check-runs?per_page=100`)).check_runs as any[]

  // Открытый, а если его нет — влитый PR из ветки; с проверками последнего коммита вместо пайплайна
  async function mrFor(repo: string, branch: string) {
    const owner = repo.split('/')[0]
    const list = await gh<any[]>(`/repos/${repo}/pulls?state=all&head=${encodeURIComponent(`${owner}:${branch}`)}`)
    const found = list.find(p => p.state === 'open') ?? list.find(p => p.merged_at)
    if (!found) return
    const status = checksStatus(await checkRuns(repo, found.head.sha))
    return toMR(found, status && { id: found.head.sha, status, url: `${found.html_url}/checks` })
  }

  const defaultBranch = async (repo: string) => (await gh(`/repos/${repo}`)).default_branch as string

  const createMR = async (repo: string, mr: { source: string, target: string, title: string, description: string }) =>
    toMR(await gh(`/repos/${repo}/pulls`, 'POST', { head: mr.source, base: mr.target, title: mr.title, body: mr.description }))

  // Хвосты логов упавших job'ов GitHub Actions — для агента. У Actions id check run совпадает с id job'а
  async function failedJobLogs(repo: string, sha: number | string) {
    const failed = (await checkRuns(repo, String(sha))).filter(r => isFailedCheck(r.conclusion) && r.app?.slug === 'github-actions')
    const logs = await Promise.all(failed.map(async run =>
      `## ${run.name}\n${run.html_url}\n\n\`\`\`\n${jobLogTail(await gh<string>(`/repos/${repo}/actions/jobs/${run.id}/logs`))}\n\`\`\``))
    return logs.join('\n\n')
  }

  const openMRs = async (repo: string) =>
    toPage((await gh<any[]>(`/repos/${repo}/pulls?state=open&sort=updated&direction=desc&per_page=${FORGE_PAGE}`)).map(p => toMR(p)))

  // Список issues GitHub включает и PR — их отсекаем
  async function openIssues(repo: string) {
    const raw = await gh<any[]>(`/repos/${repo}/issues?state=open&sort=updated&per_page=${FORGE_PAGE}`)
    return toPage(raw.filter(i => !i.pull_request).map((i): RepoIssue => ({
      iid: i.number,
      title: i.title,
      url: i.html_url,
      author: i.user?.login ?? '',
      labels: i.labels.map((l: any) => typeof l === 'string' ? l : l.name),
      updated: i.updated_at,
    })), raw.length)
  }

  return { mrFor, defaultBranch, createMR, failedJobLogs, openMRs, openIssues }
}
