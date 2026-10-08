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
