import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'

// git: false — папка без git: один «worktree» — она сама, без веток
export interface Worktree { path: string, branch: string | null, git: boolean }

const projects = persisted<string[]>('projects', [])
const hidden = persisted<string[]>('hiddenProjects', [])
const collapsed = persisted<string[]>('collapsedProjects', [])
const worktrees = ref<Record<string, Worktree[]>>({})
const activeWorktree = ref<string>()
// От какой ветки создан worktree (по пути) — от неё ChangesPanel считает diff
const bases = persisted<Record<string, string>>('worktree-bases', {})
// Последняя выбранная база по репозиторию — подставляется в следующий раз
const lastBase = persisted<Record<string, string>>('last-bases', {})

export const baseName = (path: string) => path.split('/').pop() ?? path
export const worktreeName = (wt: Worktree) => wt.git ? wt.branch ?? 'detached HEAD' : 'без git'

const without = (list: string[], repo: string) => list.filter(p => p !== repo)

export function useProjects() {
  const { closeTerminalsIn } = useTerminals()
  const { closeFilesIn } = useEditor()

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
      if (wt) return `${baseName(repo)} / ${worktreeName(wt)}`
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

  // Любая папка внутри репозитория → проект по корню, папка без git — сама; ошибки git пробрасываются
  async function addPath(dir: string) {
    const repo = await invoke<string>('git_root', { path: dir })
    if (!projects.value.includes(repo)) projects.value.push(repo)
    hidden.value = without(hidden.value, repo)
    await refresh(repo)
    activeWorktree.value = repo
    return repo
  }

  async function addProject() {
    const dir = await open({ directory: true, title: 'Выберите папку проекта' })
    if (dir) await addPath(dir).catch(e => toast.error(String(e)))
  }

  // Убирает только из списка: на диске ничего не трогаем
  function removeProject(repo: string) {
    worktrees.value[repo]?.forEach((wt) => {
      closeTerminalsIn(wt.path)
      closeFilesIn(wt.path)
    })
    if (active.value?.repo === repo) activeWorktree.value = undefined
    projects.value = without(projects.value, repo)
    hidden.value = without(hidden.value, repo)
    collapsed.value = without(collapsed.value, repo)
    delete worktrees.value[repo]
  }

  const toggle = (list: Ref<string[]>, repo: string) => {
    list.value = list.value.includes(repo) ? without(list.value, repo) : [...list.value, repo]
  }
  const toggleHidden = (repo: string) => toggle(hidden, repo)
  const toggleCollapsed = (repo: string) => toggle(collapsed, repo)

  // Ошибки git пробрасываются: их показывает диалог, который вызвал
  async function createWorktree(repo: string, branch: string, base: string) {
    const path = await invoke<string>('git_worktree_add', { repo, branch, base })
    bases.value[path] = base
    lastBase.value[repo] = base
    await refresh(repo)
    activeWorktree.value = path
    return path
  }

  async function removeWorktree(repo: string, path: string, force: boolean) {
    await invoke('git_worktree_remove', { repo, path, force })
    closeTerminalsIn(path)
    closeFilesIn(path)
    delete bases.value[path]
    if (activeWorktree.value === path) activeWorktree.value = repo
    await refresh(repo)
  }

  return { projects, hidden, collapsed, worktrees, activeWorktree, active, bases, lastBase, label, refresh, refreshAll, addPath, addProject, removeProject, toggleHidden, toggleCollapsed, createWorktree, removeWorktree }
}
