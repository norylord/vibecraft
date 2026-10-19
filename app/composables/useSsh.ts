import type { SshServer } from '@/utils/ssh'
import { invoke } from '@tauri-apps/api/core'

const servers = persisted<SshServer[]>('ssh-servers', [])

// Пароль — запись ssh-<id> в ~/.vibecraft/credentials.json (integration_*): во webview приходит только факт наличия
const passwords = ref<Record<string, boolean>>({})
const secretId = (id: string) => `ssh-${id}`

async function reload(id: string) {
  const stored = await invoke<{ hasToken: boolean } | null>('integration_get', { id: secretId(id) })
  return (passwords.value[id] = !!stored?.hasToken)
}
watch(() => servers.value.map(s => s.id), ids => ids.filter(id => !(id in passwords.value)).forEach(reload), { immediate: true })

const local = () => invoke<{ keys: string[], config: string }>('ssh_local')

export function useSsh() {
  const view = useView()
  const { activeWorktree } = useProjects()
  const { openSsh } = useTerminals()

  // password: пустой — оставить сохранённый. Пароль пишем первым: при ошибке сервер не меняется
  async function save(server: SshServer, password: string) {
    if (password) await invoke('integration_save', { id: secretId(server.id), url: '', token: password })
    const i = servers.value.findIndex(s => s.id === server.id)
    if (i < 0) servers.value.push(server)
    else servers.value[i] = server
    await reload(server.id)
  }

  async function forget(id: string) {
    await invoke('integration_delete', { id: secretId(id) })
    await reload(id)
  }

  async function remove(server: SshServer) {
    await invoke('integration_delete', { id: secretId(server.id) })
    servers.value = servers.value.filter(s => s.id !== server.id)
    delete passwords.value[server.id]
  }

  // Вкладкой в текущем worktree (без него — в домашней папке); наличие пароля — свежее, из credentials.json
  async function connect(server: SshServer) {
    view.value = 'workspace'
    openSsh(activeWorktree.value, server.name.trim() || server.host, sshCommand(server, await reload(server.id)))
  }

  // Только псевдонимы: HostName, User, IdentityFile ssh возьмёт из ~/.ssh/config сам
  async function importConfig() {
    const known = new Set(servers.value.map(s => s.host))
    const hosts = configHosts((await local()).config).filter(h => !known.has(h))
    servers.value.push(...hosts.map(host => ({ id: crypto.randomUUID(), name: host, host })))
    return hosts.length
  }

  return { servers, passwords, save, forget, remove, connect, local, importConfig }
}
