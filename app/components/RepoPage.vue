<script setup lang="ts">
import type { MergeRequest } from '@/composables/useGitLab'
import { CircleDotIcon, ExternalLinkIcon, FolderGit2Icon, GitPullRequestIcon, HistoryIcon, RefreshCwIcon, SquareTerminalIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener } from '@vueuse/core'

interface Commit { hash: string, short: string, subject: string, author: string, date: string, refs: string[] }

// ponytail: последние 200 коммитов без подгрузки — догружать, если понадобится история глубже
const COMMITS_LIMIT = 200

const view = useView()
const tab = useRepoTab()
const { active, worktrees, activeWorktree } = useProjects()
const { forge, api, terms, current, loadRemote, loadLists } = useForge()

const commits = ref<Commit[]>([])
const loading = ref(false)
const error = ref<string>()

// История — из локального git: работает для любого хостинга и без remote
async function loadCommits() {
  const wt = active.value?.wt
  if (!wt?.git) return
  loading.value = true
  try {
    const list = await invoke<Commit[]>('git_log', { path: wt.path, limit: COMMITS_LIMIT })
    // Пока git думал, пользователь переключил worktree
    if (wt.path !== active.value?.wt.path) return
    commits.value = list
    error.value = undefined
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    loading.value = false
  }
}
watch(() => active.value?.wt.path, () => {
  commits.value = []
  error.value = undefined
  loadCommits()
}, { immediate: true })
useEventListener(window, 'focus', loadCommits)

// У хостинга без API вкладок PR/MR и issues нет
watch(api, (a) => {
  if (!a) tab.value = 'commits'
}, { immediate: true })

const refreshing = ref(false)
async function refresh() {
  refreshing.value = true
  // remote — первым: от него зависит, чьи PR/MR и issues грузить
  await loadRemote()
  await Promise.all([loadCommits(), loadLists()])
  refreshing.value = false
}

// Worktree этого репозитория на ветке PR/MR — открыть его терминалы
const worktreeOf = (mr: MergeRequest) => active.value && worktrees.value[active.value.repo]?.find(w => w.branch === mr.source)?.path

function openWorktree(path: string) {
  activeWorktree.value = path
  view.value = 'workspace'
}

const listTitle = (what: string) => current.value?.error ? `Не удалось загрузить ${what}` : current.value ? `Открытых ${what} нет` : 'Загружаю…'
const rowClass = 'flex items-center gap-3 border-b px-3 py-2 animate-in fade-in slide-in-from-bottom-1 fill-mode-backwards'
const rowDelay = (i: number) => ({ animationDelay: `${Math.min(i, 15) * 20}ms` })
</script>

<template>
  <SidebarInset class="min-w-0 overflow-hidden animate-in fade-in duration-200">
    <AppHeader>
      <template v-if="active">
        <span class="text-foreground">{{ baseName(active.repo) }}</span> / {{ worktreeName(active.wt) }}
      </template>
      <template v-else>
        Проект не выбран
      </template>
      <template #actions>
        <Tooltip v-if="forge">
          <TooltipTrigger as-child>
            <Button variant="ghost" size="icon-sm" @click="openExternal(forge.url)">
              <ExternalLinkIcon />
              <span class="sr-only">Открыть в {{ forge.name }}</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>Открыть в {{ forge.name }}</TooltipContent>
        </Tooltip>
        <Button variant="ghost" size="icon-sm" :disabled="refreshing || !active?.wt.git" @click="refresh">
          <Spinner v-if="refreshing" />
          <RefreshCwIcon v-else />
          <span class="sr-only">Обновить</span>
        </Button>
      </template>
    </AppHeader>

    <Empty v-if="!active?.wt.git" class="flex-1">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <FolderGit2Icon />
        </EmptyMedia>
        <EmptyTitle>{{ active ? 'Папка без git' : 'Проект не выбран' }}</EmptyTitle>
        <EmptyDescription>
          {{ active ? 'Коммиты, PR и issues появятся у git-репозитория' : 'Выберите worktree в сайдбаре' }}
        </EmptyDescription>
      </EmptyHeader>
    </Empty>

    <div v-else class="flex min-h-0 flex-1 flex-col">
      <Tabs v-model="tab" class="p-2">
        <TabsList variant="line">
          <TabsTrigger value="commits">
            Коммиты
          </TabsTrigger>
          <template v-if="api">
            <TabsTrigger value="mrs">
              {{ terms.mrName }}s
            </TabsTrigger>
            <TabsTrigger value="issues">
              Issues
            </TabsTrigger>
          </template>
        </TabsList>
      </Tabs>
      <Separator />

      <div class="min-h-0 flex-1 overflow-auto">
        <!-- Коммиты ветки worktree: ссылка на коммит — если хостинг известен -->
        <template v-if="tab === 'commits'">
          <Empty v-if="error || !commits.length" class="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <HistoryIcon />
              </EmptyMedia>
              <EmptyTitle>{{ error ? 'Не удалось получить коммиты' : loading ? 'Загружаю…' : 'Коммитов пока нет' }}</EmptyTitle>
              <EmptyDescription v-if="error">
                {{ error }}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
          <div v-for="(c, i) in commits" :key="c.hash" :class="rowClass" :style="rowDelay(i)">
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span class="truncate text-sm select-text">{{ c.subject }}</span>
              <span class="flex items-center gap-2 text-xs text-muted-foreground">
                <span class="font-mono select-text">{{ c.short }}</span>
                <Badge v-for="r in c.refs" :key="r" variant="outline" class="font-mono">{{ r }}</Badge>
                <span class="truncate">{{ c.author }}</span>
                <span class="ml-auto shrink-0">{{ ago(c.date) }}</span>
              </span>
            </div>
            <Button v-if="forge && commitUrl(forge, c.hash)" variant="ghost" size="icon-sm" @click="openExternal(commitUrl(forge, c.hash)!)">
              <ExternalLinkIcon />
              <span class="sr-only">Открыть в {{ forge.name }}</span>
            </Button>
          </div>
          <p v-if="commits.length >= COMMITS_LIMIT" class="px-3 py-2 text-xs text-muted-foreground">
            Показаны последние {{ COMMITS_LIMIT }} коммитов
          </p>
        </template>

        <template v-else-if="tab === 'mrs'">
          <Empty v-if="!current?.mrs?.items.length" class="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <GitPullRequestIcon />
              </EmptyMedia>
              <EmptyTitle>{{ listTitle(terms.mr) }}</EmptyTitle>
              <EmptyDescription v-if="current?.error">
                {{ current.error }}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
          <div v-for="(mr, i) in current?.mrs?.items" :key="mr.url" :class="rowClass" :style="rowDelay(i)">
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span class="flex items-center gap-2 text-xs text-muted-foreground">
                <span class="font-mono">{{ terms.ref }}{{ mr.iid }}</span>
                <Badge v-if="mr.draft" variant="outline">Draft</Badge>
                <span class="truncate">{{ mr.author }}</span>
                <span class="ml-auto shrink-0">{{ ago(mr.updated) }}</span>
              </span>
              <span class="truncate text-sm">{{ mr.title }}</span>
              <span class="truncate font-mono text-xs text-muted-foreground">{{ mr.source }} → {{ mr.target }}</span>
            </div>
            <Button v-if="worktreeOf(mr)" variant="outline" size="sm" @click="openWorktree(worktreeOf(mr)!)">
              <SquareTerminalIcon data-icon="inline-start" />
              Worktree
            </Button>
            <Button variant="ghost" size="icon-sm" @click="openExternal(mr.url)">
              <ExternalLinkIcon />
              <span class="sr-only">Открыть в {{ forge?.name }}</span>
            </Button>
          </div>
          <p v-if="current?.mrs?.more" class="px-3 py-2 text-xs text-muted-foreground">
            Показаны {{ current.mrs.items.length }} последних — остальные в {{ forge?.name }}
          </p>
        </template>

        <template v-else>
          <Empty v-if="!current?.issues?.items.length" class="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <CircleDotIcon />
              </EmptyMedia>
              <EmptyTitle>{{ listTitle('issues') }}</EmptyTitle>
              <EmptyDescription v-if="current?.error">
                {{ current.error }}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
          <div v-for="(issue, i) in current?.issues?.items" :key="issue.url" :class="rowClass" :style="rowDelay(i)">
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span class="flex items-center gap-2 text-xs text-muted-foreground">
                <span class="font-mono">#{{ issue.iid }}</span>
                <Badge v-for="label in issue.labels" :key="label" variant="outline">{{ label }}</Badge>
                <span class="truncate">{{ issue.author }}</span>
                <span class="ml-auto shrink-0">{{ ago(issue.updated) }}</span>
              </span>
              <span class="truncate text-sm">{{ issue.title }}</span>
            </div>
            <Button variant="ghost" size="icon-sm" @click="openExternal(issue.url)">
              <ExternalLinkIcon />
              <span class="sr-only">Открыть в {{ forge?.name }}</span>
            </Button>
          </div>
          <p v-if="current?.issues?.more" class="px-3 py-2 text-xs text-muted-foreground">
            Показаны {{ current.issues.items.length }} последних — остальные в {{ forge?.name }}
          </p>
        </template>
      </div>
    </div>
  </SidebarInset>
</template>
