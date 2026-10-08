<script setup lang="ts">
import type { Issue } from '@/composables/useYouTrack'
import { ExternalLinkIcon, TicketIcon } from '@lucide/vue'

const { active } = useProjects()
const { configured } = useIntegrations()
const { links, issue: fetchIssue, issueUrl } = useYouTrack()

const hasYouTrack = computed(() => configured.value.some(i => i.id === 'youtrack'))
const linkedId = computed(() => active.value ? links.value[active.value.wt.path] : undefined)

const issue = ref<Issue>()
const error = ref<string>()
watch([linkedId, hasYouTrack], async ([id, on]) => {
  issue.value = undefined
  error.value = undefined
  if (!id || !on) return
  try {
    issue.value = await fetchIssue(id)
  }
  catch (e) {
    error.value = String(e)
  }
}, { immediate: true })
</script>

<template>
  <Empty v-if="!issue" class="h-full">
    <EmptyHeader>
      <EmptyMedia variant="icon">
        <TicketIcon />
      </EmptyMedia>
      <EmptyTitle>
        {{ !linkedId ? 'Задача не привязана' : error ? 'Не удалось загрузить задачу' : !hasYouTrack ? 'YouTrack не подключён' : 'Загружаю…' }}
      </EmptyTitle>
      <EmptyDescription>
        {{ !linkedId ? 'Начните работу из YouTrack — задача привяжется к worktree' : error ?? linkedId }}
      </EmptyDescription>
    </EmptyHeader>
  </Empty>

  <div v-else class="flex h-full flex-col gap-3">
    <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
      <span class="font-mono">{{ issue.id }}</span>
      <Badge v-if="issue.state" variant="outline">{{ issue.state }}</Badge>
      <Badge v-if="issue.priority" variant="outline">{{ issue.priority }}</Badge>
    </div>
    <h3 class="text-sm font-medium select-text">
      {{ issue.summary }}
    </h3>
    <Button variant="outline" @click="openExternal(issueUrl(issue.id))">
      <ExternalLinkIcon data-icon="inline-start" />
      Открыть в YouTrack
    </Button>
    <!-- Описание обычным текстом: HTML из трекера не рендерим -->
    <div class="min-h-0 flex-1 overflow-auto text-xs/relaxed whitespace-pre-wrap text-muted-foreground select-text">
      {{ issue.description || 'Без описания' }}
    </div>
  </div>
</template>
