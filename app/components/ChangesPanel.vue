<script setup lang="ts">
import { ArrowUpIcon, ChevronRightIcon, GitCommitHorizontalIcon, GitCompareIcon, RefreshCwIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener } from '@vueuse/core'
import { toast } from 'vue-sonner'
import { cn } from '@/lib/utils'

const { active, worktrees } = useProjects()
const { statusIn } = useTerminals()

const diff = ref('')
// ahead: null — ветку ещё не публиковали; remote: null — публиковать некуда
const status = ref<{ changes: number, ahead: number | null, remote: string | null }>({ changes: 0, ahead: null, remote: null })
const loading = ref(false)
const error = ref<string>()
const files = computed(() => parseDiff(diff.value))
const totals = computed(() => files.value.reduce(
  (t, f) => ({ added: t.added + f.added, removed: t.removed + f.removed }),
  { added: 0, removed: 0 },
))

// База — ветка основного worktree (от неё создаются новые); у самого основного — только незакоммиченное
const base = computed(() => {
  const main = active.value && worktrees.value[active.value.repo]?.[0]
  return main && main.path !== active.value?.wt.path ? main.branch ?? undefined : undefined
})

async function load() {
  const path = active.value?.wt.path
  if (!path) return
  loading.value = true
  try {
    const [d, s] = await Promise.all([
      invoke<string>('git_diff', { path, base: base.value }),
      invoke<typeof status.value>('git_status', { path }),
    ])
    // Пока git думал, пользователь переключил worktree — этот результат уже не нужен
    if (path !== active.value?.wt.path) return
    diff.value = d
    status.value = s
    error.value = undefined
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    loading.value = false
  }
}

// Смена worktree и смена статуса агента в нём (закончил ход — появились правки)
watch([() => active.value?.wt.path, () => active.value && statusIn(active.value.wt.path)], load, { immediate: true })
useEventListener(window, 'focus', load)

const message = ref('')
const busy = ref<'commit' | 'push'>()
const canCommit = computed(() => !busy.value && status.value.changes > 0 && !!message.value.trim())
const canPush = computed(() => !busy.value && (status.value.ahead === null ? !!status.value.remote : status.value.ahead > 0))
const pushLabel = computed(() => {
  const { ahead, remote } = status.value
  if (ahead !== null) return ahead ? `Push ↑${ahead}` : 'Push'
  return remote ? `Опубликовать в ${remote}` : 'Нет remote'
})

async function gitAction(action: 'commit' | 'push') {
  const path = active.value?.wt.path
  if (!path || busy.value || (action === 'commit' && !canCommit.value)) return
  busy.value = action
  try {
    if (action === 'commit') {
      await invoke('git_commit', { path, message: message.value.trim() })
      message.value = ''
    }
    else {
      await invoke('git_push', { path })
    }
    toast.success(action === 'commit' ? 'Закоммичено' : 'Запушено')
    await load()
  }
  catch (e) {
    toast.error(String(e))
  }
  finally {
    busy.value = undefined
  }
}

const lineClass = (line: string) => cn('px-3 whitespace-pre', {
  'bg-success/10': line.startsWith('+'),
  'bg-destructive/10': line.startsWith('-'),
  'text-muted-foreground': line.startsWith('@@') || line.startsWith('\\'),
})
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex h-8 shrink-0 items-center gap-2 text-xs text-muted-foreground">
      <template v-if="files.length">
        <span>Файлов: {{ files.length }}</span>
        <span class="text-success">+{{ totals.added }}</span>
        <span class="text-destructive">−{{ totals.removed }}</span>
      </template>
      <span v-if="base" class="truncate">от {{ base }}</span>
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
            <GitCompareIcon />
          </EmptyMedia>
          <EmptyTitle>{{ !active ? 'Worktree не выбран' : error ? 'Не удалось получить diff' : 'Нет изменений' }}</EmptyTitle>
          <EmptyDescription>{{ error ?? 'Здесь появятся правки агента: коммиты ветки, незакоммиченное и новые файлы' }}</EmptyDescription>
        </EmptyHeader>
      </Empty>

      <!-- Большие файлы (lock-файлы и т.п.) свёрнуты, чтобы не тормозить рендер -->
      <template v-else>
        <details v-for="file in files" :key="file.path" :open="file.lines.length < 400" class="group border-b">
          <summary class="sticky top-0 flex list-none items-center gap-2 bg-background px-1 py-1.5 text-xs [&::-webkit-details-marker]:hidden">
            <ChevronRightIcon class="size-3.5 shrink-0 text-muted-foreground transition-transform group-open:rotate-90" />
            <span class="truncate font-mono">{{ file.path }}</span>
            <span class="ml-auto shrink-0 text-success">+{{ file.added }}</span>
            <span class="shrink-0 text-destructive">−{{ file.removed }}</span>
          </summary>
          <div class="overflow-x-auto font-mono text-xs/5 select-text">
            <div class="w-max min-w-full">
              <!-- В одну строку: с whitespace-pre перенос вокруг {{ }} дал бы лишний пробел -->
              <div v-for="(line, i) in file.lines" :key="i" :class="lineClass(line)">{{ line || ' ' }}</div>
            </div>
          </div>
        </details>
      </template>
    </div>

    <form v-if="active" class="flex shrink-0 flex-col gap-2 border-t pt-2" @submit.prevent="gitAction('commit')">
      <Field>
        <FieldLabel for="commit-message" class="sr-only">Сообщение коммита</FieldLabel>
        <Textarea
          id="commit-message"
          v-model="message"
          rows="2"
          placeholder="Сообщение коммита (⌘↵)"
          class="min-h-0 resize-none"
          @keydown.meta.enter.prevent="gitAction('commit')"
        />
      </Field>
      <div class="flex gap-2">
        <Button type="submit" class="flex-1" :disabled="!canCommit">
          <Spinner v-if="busy === 'commit'" data-icon="inline-start" />
          <GitCommitHorizontalIcon v-else data-icon="inline-start" />
          Закоммитить{{ status.changes ? ` (${status.changes})` : '' }}
        </Button>
        <Button type="button" variant="outline" class="flex-1" :disabled="!canPush" @click="gitAction('push')">
          <Spinner v-if="busy === 'push'" data-icon="inline-start" />
          <ArrowUpIcon v-else data-icon="inline-start" />
          {{ pushLabel }}
        </Button>
      </div>
    </form>
  </div>
</template>
