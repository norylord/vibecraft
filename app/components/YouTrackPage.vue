<script setup lang="ts">
import type { Issue } from '@/composables/useYouTrack'
import { ExternalLinkIcon, PlayIcon, RefreshCwIcon, SearchIcon, TicketIcon } from '@lucide/vue'
import { cn } from '@/lib/utils'

const { issues: fetchIssues, issueUrl, links } = useYouTrack()
const { option } = useIntegrations()

const list = ref<Issue[]>([])
const loading = ref(false)
const error = ref<string>()
const search = ref('')
const selectedId = ref<string>()
const startFor = ref<Issue>()

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  return q ? list.value.filter(i => `${i.id} ${i.summary}`.toLowerCase().includes(q)) : list.value
})
const selected = computed(() => list.value.find(i => i.id === selectedId.value))
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

</script>

<template>
  <SidebarInset class="min-w-0 overflow-hidden">
    <AppHeader>
      <span class="text-foreground">YouTrack</span> · <span class="font-mono">{{ option('youtrack', 'query') }}</span>
      <template #actions>
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
            <button
              v-for="issue in filtered"
              :key="issue.id"
              type="button"
              :class="cn('flex w-full flex-col gap-1 border-b px-3 py-2 text-left hover:bg-muted/50', issue.id === selectedId && 'bg-muted')"
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
