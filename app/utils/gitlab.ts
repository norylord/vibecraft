/**
 * Путь проекта GitLab (`group/sub/proj`) из URL remote — если remote на том же хосте, что настроенный GitLab.
 * Понимает `git@host:path.git`, `https://host/path.git`, `ssh://git@host:2222/path.git`
 */
export function gitlabProject(remote: string, gitlabUrl: string) {
  const host = new URL(gitlabUrl).hostname
  const scp = remote.match(/^[^@\s/]+@([^:/]+):(?!\/)(.+)$/)
  let remoteHost: string | undefined
  let path: string | undefined
  if (scp) {
    remoteHost = scp[1]
    path = scp[2]
  }
  else {
    try {
      const url = new URL(remote)
      remoteHost = url.hostname
      path = url.pathname
    }
    catch {
      return
    }
  }
  return remoteHost === host ? path?.replace(/^\/+|\/+$/g, '').replace(/\.git$/, '') || undefined : undefined
}

/**
 * Хвост лога job'а для агента: причина падения обычно в конце. Без ANSI-цветов, маркеров секций GitLab
 * (`section_start:…\r\x1B[0K`) и управляющих символов; из строки с `\r` — то, что осталось на экране
 */
export function jobLogTail(trace: string, lines = 150) {
  return trace
    .split('\n')
    .map(line => line.replace(/\r$/, '').split('\r').at(-1)!.replace(/\x1B\[[0-9;?]*[A-Za-z~]/g, '').replace(/[\x00-\x08\x0B-\x1F\x7F]/g, ''))
    .slice(-lines)
    .join('\n')
    .trim()
}
