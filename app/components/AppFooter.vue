<script setup lang="ts">
import type { RepoTab } from '@/composables/useForge'
import type { ForgePage } from '@/utils/forge'
import { CircleDotIcon, ExternalLinkIcon, GitPullRequestIcon, HistoryIcon } from '@lucide/vue'
import { useEventListener } from '@vueuse/core'
import { cn } from '@/lib/utils'

const view = useView()
const tab = useRepoTab()
const { active } = useProjects()
const { repo, forge, api, terms, current, loadRemote, loadLists } = useForge()

// Футер смонтирован всегда — он и держит актуальными remote и счётчики хостинга активного проекта.
// repo, а не active.repo: после git init в папке проекта путь тот же, а remote читать уже есть откуда
watch(repo, loadRemote, { immediate: true })
watch(() => api.value && forge.value?.url, loadLists, { immediate: true })
// remote могли добавить, а PR — открыть, пока окно было в фоне
useEventListener(window, 'focus', async () => {
  await loadRemote()
  loadLists()
})

const isOpen = (t: RepoTab) => view.value === 'repo' && tab.value === t
// Повторный клик по открытой вкладке — обратно к терминалам
function toggle(t: RepoTab) {
  if (isOpen(t)) {
    view.value = 'workspace'
    return
  }
  tab.value = t
  view.value = 'repo'
}

const count = (page?: ForgePage<unknown>) => page && `${page.items.length}${page.more ? '+' : ''}`
const buttonClass = (t: RepoTab) => cn('text-muted-foreground', isOpen(t) && 'bg-muted text-foreground')
</script>

<template>
  <!-- fixed: SidebarProvider — flex-строка; место под футер освобождает его pb-7 в app.vue -->
  <footer class="fixed inset-x-0 bottom-0 flex h-7 items-center gap-1 overflow-hidden px-3 text-xs whitespace-nowrap text-muted-foreground">
    <!-- В узком окне (зум, пол-экрана) обрезаются лимиты, а не кнопки проекта -->
    <AgentUsage class="min-w-0" />
    <div v-if="active?.wt.git" class="flex shrink-0 items-center gap-1 not-first:ml-2 not-first:border-l not-first:pl-2">
      <Tooltip v-if="forge">
        <TooltipTrigger as-child>
          <Button variant="ghost" size="sm" class="text-muted-foreground" @click="openExternal(forge.url)">
            {{ forge.name }}
            <ExternalLinkIcon data-icon="inline-end" />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="top">
          {{ forge.url }}
        </TooltipContent>
      </Tooltip>
      <Button variant="ghost" size="sm" :class="buttonClass('commits')" @click="toggle('commits')">
        <HistoryIcon data-icon="inline-start" />
        Коммиты
      </Button>
      <template v-if="api">
        <Button variant="ghost" size="sm" :class="buttonClass('mrs')" @click="toggle('mrs')">
          <GitPullRequestIcon data-icon="inline-start" />
          {{ terms.mr }}
          <span class="tabular-nums">{{ count(current?.mrs) }}</span>
        </Button>
        <Button variant="ghost" size="sm" :class="buttonClass('issues')" @click="toggle('issues')">
          <CircleDotIcon data-icon="inline-start" />
          Issues
          <span class="tabular-nums">{{ count(current?.issues) }}</span>
        </Button>
      </template>
    </div>
  </footer>
</template>
