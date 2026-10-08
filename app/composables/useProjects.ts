import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'

export interface Worktree { path: string, branch: string | null }

const projects = persisted<string[]>('projects', [])
const worktrees = ref<Record<string, Worktree[]>>({})
const activeWorktree = ref<string>()

export const baseName = (path: string) => path.split('/').pop() ?? path

export function useProjects() {
  const { closeTerminalsIn } = useTerminals()

  const active = computed(() => {
    for (const repo of projects.value) {
      const wt = worktrees.value[repo]?.find(w => w.path === activeWorktree.value)
      if (wt) return { repo, wt }
    }
  })

  // «проект / ветка» по пути worktree — для уведомлений
  function label(path: string) {
    for (const repo of projects.value) {
      const wt = worktrees.value[repo]?.find(w => w.path === path)
      if (wt) return `${baseName(repo)} / ${wt.branch ?? 'detached HEAD'}`
    }
    return baseName(path)
  }

  async function refresh(repo: string) {
    try {
      worktrees.value[repo] = await invoke<Worktree[]>('git_worktrees', { repo })
    }
    catch (e) {
      toast.error(`${baseName(repo)}: ${e}`)
    }
  }

  const refreshAll = () => Promise.all(projects.value.map(refresh))

  // Любая папка внутри репозитория → проект по корню; ошибки git пробрасываются
  async function addPath(dir: string) {
    const repo = await invoke<string>('git_root', { path: dir })
    if (!projects.value.includes(repo)) projects.value.push(repo)
    await refresh(repo)
    activeWorktree.value = repo
    return repo
  }

  async function addProject() {
    const dir = await open({ directory: true, title: 'Выберите git-репозиторий' })
    if (dir) await addPath(dir).catch(e => toast.error(String(e)))
  }

  // Убирает только из списка: на диске ничего не трогаем
  function removeProject(repo: string) {
    worktrees.value[repo]?.forEach(wt => closeTerminalsIn(wt.path))
    if (active.value?.repo === repo) activeWorktree.value = undefined
    projects.value = projects.value.filter(p => p !== repo)
    delete worktrees.value[repo]
  }

  // Ошибки git пробрасываются: их показывает диалог, который вызвал
  async function createWorktree(repo: string, branch: string) {
    const path = await invoke<string>('git_worktree_add', { repo, branch })
    await refresh(repo)
    activeWorktree.value = path
    return path
  }

  async function removeWorktree(repo: string, path: string, force: boolean) {
    await invoke('git_worktree_remove', { repo, path, force })
    closeTerminalsIn(path)
    if (activeWorktree.value === path) activeWorktree.value = repo
    await refresh(repo)
  }

  return { projects, worktrees, activeWorktree, active, label, refresh, refreshAll, addPath, addProject, removeProject, createWorktree, removeWorktree }
}
