<script setup lang="ts">
import { ChevronRightIcon, FileIcon, FolderIcon, FolderTreeIcon, RefreshCwIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener } from '@vueuse/core'
import { cn } from '@/lib/utils'

const { active } = useProjects()
const { statusIn, activeTab } = useTerminals()
const { openFile } = useEditor()

const files = ref<string[]>([])
const loading = ref(false)
const error = ref<string>()
const expanded = ref(new Set<string>())

async function load() {
  const root = active.value?.wt.path
  if (!root) return
  loading.value = true
  try {
    const list = await invoke<string[]>('fs_list', { root })
    if (root !== active.value?.wt.path) return
    files.value = list
    error.value = undefined
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    loading.value = false
  }
}

// Другой worktree — своё дерево, свёрнутое
watch(() => active.value?.wt.path, () => {
  expanded.value = new Set()
  files.value = []
  load()
}, { immediate: true })
// Агент закончил ход — могли появиться новые файлы
watch(() => active.value && statusIn(active.value.wt.path), load)
useEventListener(window, 'focus', load)

interface Dir { dirs: Map<string, Dir>, files: string[] }

const tree = computed(() => {
  const root: Dir = { dirs: new Map(), files: [] }
  for (const file of files.value) {
    const parts = file.split('/')
    let dir = root
    for (const part of parts.slice(0, -1)) {
      let next = dir.dirs.get(part)
      if (!next) dir.dirs.set(part, next = { dirs: new Map(), files: [] })
      dir = next
    }
    dir.files.push(parts.at(-1)!)
  }
  return root
})

const byName = (a: string, b: string) => a.localeCompare(b)

// Плоский список видимых строк: сначала папки, потом файлы; содержимое — только у раскрытых папок
const rows = computed(() => {
  const out: { name: string, path: string, depth: number, dir: boolean, open?: boolean }[] = []
  const walk = (dir: Dir, prefix: string, depth: number) => {
    for (const name of [...dir.dirs.keys()].sort(byName)) {
      const path = prefix + name
      const open = expanded.value.has(path)
      out.push({ name, path, depth, dir: true, open })
      if (open) walk(dir.dirs.get(name)!, `${path}/`, depth + 1)
    }
    for (const name of [...dir.files].sort(byName)) out.push({ name, path: prefix + name, depth, dir: false })
  }
  walk(tree.value, '', 0)
  return out
})

function toggle(path: string) {
  if (!expanded.value.delete(path)) expanded.value.add(path)
}

const isOpen = (path: string) => activeTab.value === `file:${active.value?.wt.path}/${path}`
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex h-8 shrink-0 items-center gap-2 text-xs text-muted-foreground">
      <span v-if="files.length">Файлов: {{ files.length }}</span>
      <Button variant="ghost" size="icon-xs" class="ml-auto" :disabled="loading || !active" @click="load">
        <Spinner v-if="loading" />
        <RefreshCwIcon v-else />
        <span class="sr-only">Обновить</span>
      </Button>
    </div>

    <div class="min-h-0 flex-1 overflow-auto">
      <Empty v-if="!active || error || !files.length" class="h-full">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <FolderTreeIcon />
          </EmptyMedia>
          <EmptyTitle>{{ !active ? 'Worktree не выбран' : error ? 'Не удалось получить файлы' : 'Нет файлов' }}</EmptyTitle>
          <EmptyDescription v-if="error">{{ error }}</EmptyDescription>
        </EmptyHeader>
      </Empty>

      <template v-else>
        <button
          v-for="row in rows"
          :key="row.path"
          type="button"
          :class="cn('flex w-full items-center gap-1.5 rounded-sm py-0.5 pr-2 text-left text-xs hover:bg-muted/50', isOpen(row.path) && 'bg-muted')"
          :style="{ paddingLeft: `${row.depth * 12 + 4}px` }"
          :title="row.path"
          @click="row.dir ? toggle(row.path) : openFile(active!.wt.path, row.path)"
        >
          <ChevronRightIcon v-if="row.dir" :class="cn('size-3.5 shrink-0 text-muted-foreground transition-transform', row.open && 'rotate-90')" />
          <span v-else class="size-3.5 shrink-0" />
          <component :is="row.dir ? FolderIcon : FileIcon" class="size-3.5 shrink-0 text-muted-foreground" />
          <span class="truncate">{{ row.name }}</span>
        </button>
      </template>
    </div>
  </div>
</template>
