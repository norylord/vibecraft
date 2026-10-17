export type ForgeKind = 'github' | 'gitlab' | 'bitbucket' | 'other'

/** Хостинг репозитория: от него зависят кнопки футера, вкладки страницы проекта и PR/MR в правой панели */
export interface Forge {
  kind: ForgeKind
  name: string
  host: string
  // owner/repo; у GitLab — с подгруппами
  path: string
  // Страница репозитория
  url: string
  // Есть API: GitHub (токен из настроек или gh CLI) или GitLab на хосте настроенной интеграции
  api: boolean
}

// mr / mrName — как здесь называют запрос на слияние, ref — префикс его номера, commit — путь страницы коммита
const KINDS = {
  github: { name: 'GitHub', mr: 'PR', mrName: 'Pull request', ref: '#', commit: 'commit' },
  gitlab: { name: 'GitLab', mr: 'MR', mrName: 'Merge request', ref: '!', commit: '-/commit' },
  bitbucket: { name: 'Bitbucket', mr: 'PR', mrName: 'Pull request', ref: '#', commit: 'commits' },
  other: { name: '', mr: 'MR', mrName: 'Merge request', ref: '!', commit: '' },
}
const KNOWN_HOSTS: Record<string, ForgeKind> = { 'github.com': 'github', 'gitlab.com': 'gitlab', 'bitbucket.org': 'bitbucket' }

export const forgeTerms = (forge?: Forge) => KINDS[forge?.kind ?? 'other']

// Сколько открытых PR/MR и issues загружаем за раз; если пришло столько — в счётчике «100+»
export const FORGE_PAGE = 100

// Первые FORGE_PAGE штук; more — на хостинге есть ещё
export interface ForgePage<T> { items: T[], more: boolean }

// fetched — сколько пришло до фильтрации (issues GitHub приходят вперемешку с PR)
export const toPage = <T>(items: T[], fetched = items.length): ForgePage<T> => ({ items, more: fetched >= FORGE_PAGE })

/**
 * Хост, путь и страница репозитория по URL remote: `git@host:path.git`, `https://host/path.git`,
 * `ssh://git@host:2222/path.git`. Логин и токен из https-remote в ссылку не попадают
 */
export function parseRemote(remote: string) {
  const scp = remote.match(/^[^@\s/]+@([^:/]+):(?!\/)(.+)$/)
  let host: string
  let path: string
  let origin: string
  if (scp) {
    host = scp[1]!
    path = scp[2]!
    origin = `https://${host}`
  }
  else {
    try {
      const url = new URL(remote)
      host = url.hostname
      path = url.pathname
      // У ssh:// порт — не веб-сервера
      origin = url.protocol.startsWith('http') ? url.origin : `https://${host}`
    }
    catch {
      return
    }
  }
  path = path.replace(/^\/+|\/+$/g, '').replace(/\.git$/, '')
  return host && path ? { host, path, url: `${origin}/${path}` } : undefined
}

const hostOf = (url?: string) => {
  try {
    return url ? new URL(url).hostname : undefined
  }
  catch {}
}

/**
 * Хостинг по remote. Свой GitLab и GitHub Enterprise узнаём по адресам настроенных интеграций.
 * ponytail: ssh-алиасы (`git@github-work:…`) не распознаются — понадобится чтение ~/.ssh/config
 */
export function forgeOf(remote: string, integrations: { github?: string, gitlab?: string }): Forge | undefined {
  const r = parseRemote(remote)
  if (!r) return
  const github = hostOf(integrations.github)?.replace(/^api\./, '') ?? 'github.com'
  const gitlab = hostOf(integrations.gitlab)
  const kind = r.host === github ? 'github' : r.host === gitlab ? 'gitlab' : KNOWN_HOSTS[r.host] ?? 'other'
  return { kind, name: KINDS[kind].name || r.host, ...r, api: r.host === github || r.host === gitlab }
}

export const commitUrl = (forge: Forge, hash: string) =>
  KINDS[forge.kind].commit ? `${forge.url}/${KINDS[forge.kind].commit}/${hash}` : undefined

const FAILED = ['failure', 'timed_out', 'startup_failure', 'action_required']

/** Сводный статус check runs коммита (GitHub Actions и др.) в терминах пайплайна GitLab — их понимает панель PR/MR */
export function checksStatus(runs: { status: string, conclusion: string | null }[]) {
  if (!runs.length) return
  if (runs.some(r => r.status !== 'completed')) return 'running'
  if (runs.some(r => FAILED.includes(r.conclusion ?? ''))) return 'failed'
  if (runs.some(r => r.conclusion === 'cancelled')) return 'canceled'
  if (runs.every(r => r.conclusion === 'skipped' || r.conclusion === 'neutral')) return 'skipped'
  return 'success'
}

export const isFailedCheck = (conclusion: string | null) => FAILED.includes(conclusion ?? '')
