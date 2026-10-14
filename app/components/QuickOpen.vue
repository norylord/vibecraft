<script setup lang="ts">
import { FileIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'

const open = defineModel<boolean>('open', { required: true })
const { active } = useProjects()
const { openFile } = useEditor()

const files = ref<string[]>([])
const query = ref('')

watch(open, async (isOpen) => {
  if (!isOpen) return
  query.value = ''
  const root = active.value?.wt.path
  files.value = root ? await invoke<string[]>('fs_list', { root }).catch(() => []) : []
})

// В репозитории тысячи файлов — рендерим первые 50: сначала совпадения в имени, потом в пути, короткие выше.
// Совпадение — подстрока полного пути, как и у фильтра Command (он ищет по тексту пункта)
const matches = computed(() => {
  const q = query.value.toLowerCase()
  if (!q) return files.value.slice(0, 50)
  const found: { file: string, score: number }[] = []
  for (const file of files.value) {
    const lower = file.toLowerCase()
    const at = lower.indexOf(q)
    if (at === -1) continue
    found.push({ file, score: (at > lower.lastIndexOf('/') ? 0 : 1000) + file.length })
  }
  return found.sort((a, b) => a.score - b.score).slice(0, 50).map(f => f.file)
})

function select(file: string) {
  open.value = false
  if (active.value) openFile(active.value.wt.path, file)
}
</script>

<template>
  <CommandDialog v-model:open="open" title="Открыть файл" description="Поиск по файлам worktree">
    <CommandInput placeholder="Имя или путь файла…" @input="query = ($event.target as HTMLInputElement).value" />
    <CommandList>
      <CommandEmpty>Файл не найден</CommandEmpty>
      <CommandGroup>
        <CommandItem v-for="file in matches" :key="file" :value="file" @select="select(file)">
          <FileIcon />
          <!-- Текст пункта — полный путь: по нему фильтрует Command -->
          <span class="truncate"><span class="text-muted-foreground">{{ file.slice(0, file.lastIndexOf('/') + 1) }}</span>{{ baseName(file) }}</span>
        </CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>
</template>
