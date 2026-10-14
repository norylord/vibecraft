import { ask } from '@tauri-apps/plugin-dialog'

// Открытый файл — вкладка рядом с терминалами своего worktree. path — относительно root (корня worktree)
export interface FileTab {
  key: string
  root: string
  path: string
  dirty: boolean
}

// ponytail: открытые файлы не переживают перезапуск — сохранять в persisted, если понадобится
const files = ref<FileTab[]>([])

export function useEditor() {
  const { activeTab } = useTerminals()
  const view = useView()

  function openFile(root: string, path: string) {
    const key = `file:${root}/${path}`
    if (!files.value.some(f => f.key === key)) files.value.push({ key, root, path, dirty: false })
    view.value = 'workspace'
    activeTab.value = key
  }

  async function closeFile(file: FileTab) {
    if (file.dirty && !await ask(`Несохранённые изменения в ${baseName(file.path)} будут потеряны`, { title: 'Закрыть файл?', kind: 'warning', okLabel: 'Закрыть', cancelLabel: 'Отмена' })) return
    files.value = files.value.filter(f => f.key !== file.key)
  }

  // Worktree удалён — его файлов больше нет
  function closeFilesIn(root: string) {
    files.value = files.value.filter(f => f.root !== root)
  }

  return { files, openFile, closeFile, closeFilesIn }
}
