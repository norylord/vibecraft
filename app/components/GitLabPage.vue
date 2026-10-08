<script setup lang="ts">
import type { GitLabProject, MergeRequest } from '@/composables/useGitLab'
import { DownloadIcon, ExternalLinkIcon, FolderGit2Icon, GitMergeIcon, RefreshCwIcon, SearchIcon, SquareTerminalIcon } from '@lucide/vue'
import { toast } from 'vue-sonner'

const { myMRs, projects: fetchProjects, cloneDir, clonePath, pickCloneDir, clone } = useGitLab()
const { projects: localProjects, worktrees, activeWorktree } = useProjects()
const view = useView()

const kind = ref<'created' | 'review' | 'projects'>('created')
const list = ref<MergeRequest[]>([])
const repos = ref<GitLabProject[]>([])
const search = ref('')
const loading = ref(false)
const error = ref<string>()

const filteredRepos = computed(() => {
  const q = search.value.trim().toLowerCase()
  return q ? repos.value.filter(r => `${r.path} ${r.description}`.toLowerCase().includes(q)) : repos.value
})
// Уже склонирован в папку клонирования и добавлен в Diogen
const isLocal = (r: GitLabProject) => localProjects.value.includes(clonePath(r) || '')

async function load() {
  loading.value = true
  try {
    if (kind.value === 'projects') repos.value = await fetchProjects()
    else list.value = await myMRs(kind.value)
    error.value = undefined
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    loading.value = false
  }
}
watch(kind, load, { immediate: true })

// ponytail: сопоставление по имени ветки без учёта репозитория — уточнить, если ветки начнут совпадать
function localWorktree(mr: MergeRequest) {
  for (const wts of Object.values(worktrees.value)) {
    const wt = wts.find(w => w.branch === mr.source)
    if (wt) return wt.path
  }
}

function openWorktree(path: string) {
  activeWorktree.value = path
  view.value = 'workspace'
}

const cloning = ref(new Set<string>())

async function onClone(r: GitLabProject) {
  cloning.value.add(r.path)
  try {
    const repo = await clone(r)
    if (repo) toast.success(`${r.path} склонирован`, { description: repo, action: { label: 'Открыть', onClick: () => openWorktree(repo) } })
  }
  catch (e) {
    toast.error(String(e))
  }
  finally {
    cloning.value.delete(r.path)
  }
}
</script>

<template>
  <SidebarInset class="min-w-0 overflow-hidden animate-in fade-in duration-200">
    <AppHeader>
      <span class="text-foreground">GitLab</span>
      <template #actions>
        <Button variant="ghost" size="icon-sm" :disabled="loading" @click="load">
          <Spinner v-if="loading" />
          <RefreshCwIcon v-else />
          <span class="sr-only">Обновить</span>
        </Button>
      </template>
    </AppHeader>

    <div class="flex min-h-0 flex-1 flex-col">
      <Tabs v-model="kind" class="p-2">
        <TabsList variant="line">
          <TabsTrigger value="created">
            Мои MR
          </TabsTrigger>
          <TabsTrigger value="review">
            На ревью
          </TabsTrigger>
          <TabsTrigger value="projects">
            Проекты
          </TabsTrigger>
        </TabsList>
      </Tabs>
      <div v-if="kind === 'projects'" class="flex items-center gap-2 px-2 pb-2">
        <InputGroup class="max-w-sm">
          <InputGroupAddon>
            <SearchIcon />
          </InputGroupAddon>
          <InputGroupInput v-model="search" placeholder="Поиск по проектам" />
        </InputGroup>
        <span class="ml-auto truncate text-xs text-muted-foreground" :title="cloneDir">
          Клонировать в: <span class="font-mono text-foreground">{{ cloneDir || 'спросим при первом клонировании' }}</span>
        </span>
        <Button variant="ghost" size="sm" @click="pickCloneDir">
          Изменить
        </Button>
      </div>
      <Separator />

      <div class="min-h-0 flex-1 overflow-auto">
        <!-- Проекты GitLab: клонирование одной кнопкой -->
        <template v-if="kind === 'projects'">
          <Empty v-if="error || !filteredRepos.length" class="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <FolderGit2Icon />
              </EmptyMedia>
              <EmptyTitle>{{ error ? 'Не удалось загрузить проекты' : loading ? 'Загружаю…' : 'Проектов нет' }}</EmptyTitle>
              <EmptyDescription v-if="error">
                {{ error }}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
          <div v-for="(r, i) in filteredRepos" :key="r.path" class="flex items-center gap-3 border-b px-3 py-2 animate-in fade-in slide-in-from-bottom-1 fill-mode-backwards" :style="{ animationDelay: `${Math.min(i, 15) * 20}ms` }">
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span class="flex items-center gap-2 text-xs text-muted-foreground">
                <span class="truncate font-mono">{{ r.path }}</span>
                <span class="ml-auto shrink-0">{{ ago(r.activity) }}</span>
              </span>
              <span v-if="r.description" class="truncate text-sm text-muted-foreground">{{ r.description }}</span>
            </div>
            <Button v-if="isLocal(r)" variant="outline" size="sm" @click="openWorktree(clonePath(r)!)">
              <SquareTerminalIcon data-icon="inline-start" />
              Открыть
            </Button>
            <Button v-else size="sm" :disabled="cloning.has(r.path)" @click="onClone(r)">
              <Spinner v-if="cloning.has(r.path)" data-icon="inline-start" />
              <DownloadIcon v-else data-icon="inline-start" />
              Клонировать
            </Button>
            <Button variant="ghost" size="icon-sm" @click="openExternal(r.url)">
              <ExternalLinkIcon />
              <span class="sr-only">Открыть в GitLab</span>
            </Button>
          </div>
        </template>
        <template v-else>
          <Empty v-if="error || !list.length" class="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <GitMergeIcon />
              </EmptyMedia>
              <EmptyTitle>{{ error ? 'Не удалось загрузить MR' : loading ? 'Загружаю…' : 'Открытых MR нет' }}</EmptyTitle>
              <EmptyDescription v-if="error">
                {{ error }}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
          <div v-for="(mr, i) in list" :key="mr.url" class="flex items-center gap-3 border-b px-3 py-2 animate-in fade-in slide-in-from-bottom-1 fill-mode-backwards" :style="{ animationDelay: `${Math.min(i, 15) * 20}ms` }">
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span class="flex items-center gap-2 text-xs text-muted-foreground">
                <span class="truncate font-mono">{{ mr.project }} !{{ mr.iid }}</span>
                <Badge v-if="mr.draft" variant="outline">Draft</Badge>
                <span class="ml-auto shrink-0">{{ ago(mr.updated) }}</span>
              </span>
              <span class="truncate text-sm">{{ mr.title }}</span>
              <span class="truncate font-mono text-xs text-muted-foreground">{{ mr.source }} → {{ mr.target }}</span>
            </div>
            <Button v-if="localWorktree(mr)" variant="outline" size="sm" @click="openWorktree(localWorktree(mr)!)">
              <SquareTerminalIcon data-icon="inline-start" />
              Worktree
            </Button>
            <Button variant="ghost" size="icon-sm" @click="openExternal(mr.url)">
              <ExternalLinkIcon />
              <span class="sr-only">Открыть в GitLab</span>
            </Button>
          </div>
        </template>
      </div>
    </div>
  </SidebarInset>
</template>
