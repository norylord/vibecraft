<script setup lang="ts">
import { ChevronRightIcon, CodeXmlIcon, CopyIcon, FileIcon, FilePlusIcon, FolderIcon, FolderOpenIcon, FolderTreeIcon, RefreshCwIcon, Trash2Icon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { ask } from '@tauri-apps/plugin-dialog'
import { useEventListener } from '@vueuse/core'
import { toast } from 'vue-sonner'
import { cn } from '@/lib/utils'

const { active } = useProjects()
const { statusIn, activeTab } = useTerminals()
const { openFile, closeFilesIn } = useEditor()
const { settings } = useSettings()

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

interface Row { name: string, path: string, depth: number, dir: boolean, open?: boolean }

// Плоский список видимых строк: сначала папки, потом файлы; содержимое — только у раскрытых папок
const rows = computed(() => {
  const out: Row[] = []
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

// Раскрыть папку вместе с родителями — чтобы новый файл было видно в дереве
function expandTo(dir: string) {
  const parts = dir.split('/')
  if (dir) parts.forEach((_, i) => expanded.value.add(parts.slice(0, i + 1).join('/')))
}

const rowAt = (el: Element | null) => rows.value.find(r => r.path === el?.closest<HTMLElement>('[data-path]')?.dataset.path)
const parentOf = (path: string) => path.slice(0, Math.max(0, path.lastIndexOf('/')))
// Папка строки: для файла — та, где он лежит; без строки — корень worktree
const dirOf = (row?: Row) => !row ? '' : row.dir ? row.path : parentOf(row.path)
const absolute = (path: string) => path ? `${active.value!.wt.path}/${path}` : active.value!.wt.path

// Контекстное меню: строка под правым кликом, мимо строк — корень
const menuRow = ref<Row>()

async function remove(row: Row) {
  const root = active.value!.wt.path
  const what = row.dir ? `Папка ${row.path} будет удалена со всем содержимым` : `Файл ${row.path} будет удалён`
  if (!await ask(`${what} безвозвратно`, { title: 'Удалить?', kind: 'warning', okLabel: 'Удалить', cancelLabel: 'Отмена' })) return
  try {
    await invoke('fs_delete', { root, path: row.path })
    closeFilesIn(root, row.path)
  }
  catch (e) {
    toast.error(String(e))
  }
  load()
}

const copyPath = (path: string) => navigator.clipboard.writeText(path).then(() => toast('Путь скопирован', { description: path }))

// Новый файл: папка, в которой создаём (undefined — диалог закрыт); в имени можно указать и подпапки
const createIn = ref<string>()
const newName = ref('')
const createError = ref<string>()

function startCreate(dir: string) {
  createIn.value = dir
  newName.value = ''
  createError.value = undefined
}

async function create() {
  const root = active.value!.wt.path
  const path = `${createIn.value}/${newName.value}`.split('/').filter(Boolean).join('/')
  try {
    await invoke('fs_create', { root, path })
    createIn.value = undefined
    expandTo(parentOf(path))
    await load()
    openFile(root, path)
  }
  catch (e) {
    createError.value = String(e)
  }
}

// Перетаскивание из Finder: пути файлов приходят событием окна (в HTML5 drop их нет).
// undefined — курсор не над деревом, '' — корень
const list = useTemplateRef<HTMLElement>('list')
const dropDir = ref<string>()

function dropTarget({ x, y }: { x: number, y: number }) {
  if (!active.value) return
  // macOS присылает точки окна, хоть тип и PhysicalPosition (wry не умножает на плотность экрана) —
  // в CSS-пиксели переводит только масштаб приложения
  const el = document.elementFromPoint(x / settings.value.zoom, y / settings.value.zoom)
  return el && list.value?.contains(el) ? dirOf(rowAt(el)) : undefined
}

const unlisten = getCurrentWebview().onDragDropEvent(async ({ payload }) => {
  if (payload.type === 'leave') return void (dropDir.value = undefined)
  dropDir.value = dropTarget(payload.position)
  const dir = dropDir.value
  if (payload.type !== 'drop' || dir === undefined) return
  dropDir.value = undefined
  try {
    await invoke('fs_copy', { root: active.value!.wt.path, dir, sources: payload.paths })
    expandTo(dir)
  }
  catch (e) {
    toast.error(String(e))
  }
  load()
})
onUnmounted(() => unlisten.then(off => off()))
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex h-8 shrink-0 items-center gap-2 text-xs text-muted-foreground">
      <span v-if="files.length">Файлов: {{ files.length }}</span>
      <Button variant="ghost" size="icon-xs" class="ml-auto" :disabled="!active" @click="startCreate('')">
        <FilePlusIcon />
        <span class="sr-only">Новый файл</span>
      </Button>
      <Button variant="ghost" size="icon-xs" :disabled="loading || !active" @click="load">
        <Spinner v-if="loading" />
        <RefreshCwIcon v-else />
        <span class="sr-only">Обновить</span>
      </Button>
    </div>

    <!-- Строку под меню подсвечиваем, пока оно открыто: select срабатывает раньше закрытия, menuRow в обработчиках ещё есть -->
    <ContextMenu @update:open="open => !open && (menuRow = undefined)">
      <ContextMenuTrigger as-child :disabled="!active">
        <!-- Сюда же бросают файлы из Finder: подсвечиваем корень или папку под курсором -->
        <div
          ref="list"
          :class="cn('min-h-0 flex-1 overflow-auto rounded-md', dropDir === '' && 'bg-primary/5 ring-1 ring-inset ring-primary/40')"
          @contextmenu="menuRow = rowAt($event.target as Element)"
        >
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
              :class="cn('flex w-full items-center gap-1.5 rounded-sm py-0.5 pr-2 text-left text-xs hover:bg-muted/50', (isOpen(row.path) || menuRow?.path === row.path) && 'bg-muted', row.dir && dropDir === row.path && 'bg-primary/10 ring-1 ring-inset ring-primary/40')"
              :style="{ paddingLeft: `${row.depth * 12 + 4}px` }"
              :title="row.path"
              :data-path="row.path"
              @click="row.dir ? toggle(row.path) : openFile(active!.wt.path, row.path)"
            >
              <ChevronRightIcon v-if="row.dir" :class="cn('size-3.5 shrink-0 text-muted-foreground transition-transform', row.open && 'rotate-90')" />
              <span v-else class="size-3.5 shrink-0" />
              <component :is="row.dir ? FolderIcon : FileIcon" class="size-3.5 shrink-0 text-muted-foreground" />
              <span class="truncate">{{ row.name }}</span>
            </button>
          </template>
        </div>
      </ContextMenuTrigger>
      <!-- Без возврата фокуса: после меню фокус нужен диалогу или редактору -->
      <ContextMenuContent class="w-52" @close-auto-focus.prevent>
        <ContextMenuGroup v-if="menuRow && !menuRow.dir">
          <ContextMenuItem @select="openFile(active!.wt.path, menuRow!.path)">
            <FileIcon />
            Открыть
          </ContextMenuItem>
          <ContextMenuItem @select="openPath(absolute(menuRow!.path), { app: settings.editor })">
            <CodeXmlIcon />
            Открыть в {{ settings.editor }}
          </ContextMenuItem>
        </ContextMenuGroup>
        <ContextMenuGroup>
          <ContextMenuItem @select="startCreate(dirOf(menuRow))">
            <FilePlusIcon />
            Новый файл…
          </ContextMenuItem>
          <ContextMenuItem @select="openPath(absolute(menuRow?.path ?? ''), { reveal: !!menuRow })">
            <FolderOpenIcon />
            Показать в Finder
          </ContextMenuItem>
          <ContextMenuItem v-if="menuRow" @select="copyPath(menuRow!.path)">
            <CopyIcon />
            Копировать путь
          </ContextMenuItem>
        </ContextMenuGroup>
        <template v-if="menuRow">
          <ContextMenuSeparator />
          <ContextMenuGroup>
            <ContextMenuItem variant="destructive" @select="remove(menuRow!)">
              <Trash2Icon />
              Удалить
            </ContextMenuItem>
          </ContextMenuGroup>
        </template>
      </ContextMenuContent>
    </ContextMenu>

    <Dialog :open="createIn !== undefined" @update:open="open => !open && (createIn = undefined)">
      <DialogContent class="sm:max-w-sm">
        <form class="flex flex-col gap-4" @submit.prevent="create">
          <DialogHeader>
            <DialogTitle>Новый файл</DialogTitle>
            <DialogDescription>
              {{ createIn ? `В папке ${createIn}` : 'В корне worktree' }}. Недостающие папки из имени будут созданы
            </DialogDescription>
          </DialogHeader>
          <Field :data-invalid="!!createError || undefined">
            <FieldLabel for="new-file">Имя</FieldLabel>
            <Input id="new-file" v-model="newName" placeholder="src/index.ts" autofocus :aria-invalid="!!createError || undefined" />
            <FieldError :errors="[createError]" />
          </Field>
          <DialogFooter>
            <Button type="submit" :disabled="!newName.trim()">
              Создать
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </div>
</template>
