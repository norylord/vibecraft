<script setup lang="ts">
import type { Issue } from '@/composables/useYouTrack'
import { ExternalLinkIcon, KanbanIcon, ListIcon, PlayIcon, RefreshCwIcon, SearchIcon, TicketIcon } from '@lucide/vue'
import { toast } from 'vue-sonner'
import { cn } from '@/lib/utils'

const { issues: fetchIssues, setState, issueUrl, links } = useYouTrack()
const { option } = useIntegrations()

const list = ref<Issue[]>([])
const loading = ref(false)
const error = ref<string>()
const search = ref('')
const selectedId = ref<string>()
const startFor = ref<Issue>()
const mode = persisted<'list' | 'board'>('youtrack-mode', 'list')
// ToggleGroup отдаёт пустое значение при повторном клике — не даём снять выбор
const modeModel = computed({ get: () => mode.value, set: v => v && (mode.value = v) })

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  return q ? list.value.filter(i => `${i.id} ${i.summary}`.toLowerCase().includes(q)) : list.value
})
const selected = computed(() => list.value.find(i => i.id === selectedId.value))
// Колонки — статусы в порядке YouTrack; внутри колонки сохраняется сортировка запроса.
// ponytail: только статусы, у которых есть задачи; пустые колонки — через бандл значений поля
const columns = computed(() => [...Map.groupBy(
  filtered.value.toSorted((a, b) => (a.stateOrder ?? Infinity) - (b.stateOrder ?? Infinity)),
  i => i.state ?? '',
)])
// Задачи, под которые уже заведён worktree
const linked = computed(() => new Set(Object.values(links.value)))

async function load() {
  loading.value = true
  try {
    list.value = await fetchIssues()
    error.value = undefined
    if (!selected.value) selectedId.value = list.value[0]?.id
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    loading.value = false
  }
}
onMounted(load)

const dragged = ref<Issue>()
const dropTarget = ref<string>()

function dragStart(e: DragEvent, issue: Issue) {
  e.dataTransfer?.setData('text/plain', issue.id)
  dragged.value = issue
}

// preventDefault разрешает drop; в «Без статуса» и в свою же колонку бросать нельзя
function dragOver(e: DragEvent, state: string) {
  const ok = !!state && !!dragged.value?.stateField && state !== dragged.value.state
  dropTarget.value = ok ? state : undefined
  if (ok) e.preventDefault()
}

// Оптимистично двигаем карточку, при отказе YouTrack (например, workflow запрещает переход) возвращаем
async function drop(state: string) {
  const issue = dragged.value!
  const prev = { state: issue.state, stateOrder: issue.stateOrder }
  const stateOrder = columns.value.find(([s]) => s === state)?.[1][0]?.stateOrder
  Object.assign(issue, { state, stateOrder })
  try {
    await setState(issue, state)
  }
  catch (e) {
    Object.assign(issue, prev)
    toast.error(`${issue.id}: ${e}`)
  }
}

</script>

<template>
  <SidebarInset class="min-w-0 overflow-hidden animate-in fade-in duration-200">
    <AppHeader>
      <span class="text-foreground">YouTrack</span> · <span class="font-mono">{{ option('youtrack', 'query') }}</span>
      <template #actions>
        <ToggleGroup v-model="modeModel" type="single" size="sm">
          <ToggleGroupItem value="list" aria-label="Список">
            <ListIcon />
          </ToggleGroupItem>
          <ToggleGroupItem value="board" aria-label="Доска">
            <KanbanIcon />
          </ToggleGroupItem>
        </ToggleGroup>
        <Button variant="ghost" size="icon-sm" :disabled="loading" @click="load">
          <Spinner v-if="loading" />
          <RefreshCwIcon v-else />
          <span class="sr-only">Обновить</span>
        </Button>
      </template>
    </AppHeader>

    <ResizablePanelGroup direction="horizontal" auto-save-id="diogen-youtrack" class="min-h-0 flex-1">
      <ResizablePanel :min-size="30" :default-size="55">
        <div class="flex h-full flex-col">
          <div class="p-2">
            <InputGroup>
              <InputGroupAddon>
                <SearchIcon />
              </InputGroupAddon>
              <InputGroupInput v-model="search" placeholder="Поиск по задачам" />
            </InputGroup>
          </div>
          <div class="min-h-0 flex-1 overflow-auto">
            <Empty v-if="error || !filtered.length" class="h-full">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <TicketIcon />
                </EmptyMedia>
                <EmptyTitle>{{ error ? 'Не удалось загрузить задачи' : loading ? 'Загружаю…' : 'Задач нет' }}</EmptyTitle>
                <EmptyDescription>{{ error ?? 'Запрос для списка меняется в настройках' }}</EmptyDescription>
              </EmptyHeader>
            </Empty>
            <div v-else-if="mode === 'board'" class="flex h-full gap-2 px-2 pb-2">
              <div
                v-for="[state, items] in columns"
                :key="state"
                :class="cn('flex w-64 shrink-0 flex-col rounded-lg bg-muted/50', dropTarget === state && 'ring-2 ring-ring')"
                @dragover="dragOver($event, state)"
                @drop="drop(state)"
              >
                <div class="flex items-center gap-2 px-3 py-2 text-xs font-medium">
                  {{ state || 'Без статуса' }}
                  <span class="text-muted-foreground">{{ items.length }}</span>
                </div>
                <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-2 pb-2">
                  <button
                    v-for="issue in items"
                    :key="issue.id"
                    type="button"
                    draggable="true"
                    :class="cn('flex flex-col gap-1 rounded-md border bg-background p-2 text-left hover:bg-muted/50', issue.id === selectedId && 'ring-2 ring-ring', dragged === issue && 'opacity-50')"
                    @dragstart="dragStart($event, issue)"
                    @dragend="dragged = undefined; dropTarget = undefined"
                    @click="selectedId = issue.id"
                  >
                    <span class="flex items-center gap-2 text-xs text-muted-foreground">
                      <span class="font-mono">{{ issue.id }}</span>
                      <Badge v-if="issue.priority" variant="outline">{{ issue.priority }}</Badge>
                      <Badge v-if="linked.has(issue.id)" variant="secondary">worktree</Badge>
                    </span>
                    <span class="line-clamp-3 text-sm">{{ issue.summary }}</span>
                  </button>
                </div>
              </div>
            </div>
            <template v-else>
              <button
                v-for="(issue, i) in filtered"
                :key="issue.id"
                type="button"
                :class="cn('flex w-full flex-col gap-1 border-b px-3 py-2 text-left hover:bg-muted/50 animate-in fade-in slide-in-from-bottom-1 fill-mode-backwards', issue.id === selectedId && 'bg-muted')"
                :style="{ animationDelay: `${Math.min(i, 15) * 20}ms` }"
                @click="selectedId = issue.id"
              >
                <span class="flex items-center gap-2 text-xs text-muted-foreground">
                  <span class="font-mono">{{ issue.id }}</span>
                  <Badge v-if="issue.state" variant="outline">{{ issue.state }}</Badge>
                  <Badge v-if="linked.has(issue.id)" variant="secondary">worktree</Badge>
                  <span class="ml-auto shrink-0">{{ ago(issue.updated) }}</span>
                </span>
                <span class="truncate text-sm">{{ issue.summary }}</span>
              </button>
            </template>
          </div>
        </div>
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel :min-size="25">
        <div v-if="selected" class="flex h-full flex-col">
          <div class="flex flex-col gap-3 border-b p-4">
            <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              <span class="font-mono">{{ selected.id }}</span>
              <Badge v-if="selected.state" variant="outline">{{ selected.state }}</Badge>
              <Badge v-if="selected.priority" variant="outline">{{ selected.priority }}</Badge>
            </div>
            <h2 class="font-heading text-base font-medium select-text">
              {{ selected.summary }}
            </h2>
            <div class="flex flex-wrap gap-2">
              <Button @click="startFor = selected">
                <PlayIcon data-icon="inline-start" />
                Начать
              </Button>
              <Button variant="outline" @click="openExternal(issueUrl(selected.id))">
                <ExternalLinkIcon data-icon="inline-start" />
                Открыть в YouTrack
              </Button>
            </div>
          </div>
          <!-- Описание обычным текстом: HTML из трекера не рендерим -->
          <div class="min-h-0 flex-1 overflow-auto p-4 text-sm whitespace-pre-wrap text-muted-foreground select-text">
            {{ selected.description || 'Без описания' }}
          </div>
        </div>
        <Empty v-else class="h-full">
          <EmptyHeader>
            <EmptyTitle>Выберите задачу</EmptyTitle>
          </EmptyHeader>
        </Empty>
      </ResizablePanel>
    </ResizablePanelGroup>
  </SidebarInset>

  <StartIssueDialog v-model:issue="startFor" />
</template>
