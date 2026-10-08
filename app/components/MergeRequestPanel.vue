<script setup lang="ts">
import type { MergeRequest } from '@/composables/useGitLab'
import { CircleCheckIcon, CircleDashedIcon, CircleXIcon, ExternalLinkIcon, GitMergeIcon, RefreshCwIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, useIntervalFn } from '@vueuse/core'
import { toast } from 'vue-sonner'

const { active } = useProjects()
const { configured, stored } = useIntegrations()
const { statusIn } = useTerminals()
const { projectFor, mrFor, defaultBranch, createMR } = useGitLab()
const { links, issue: fetchIssue, comment, issueUrl } = useYouTrack()
const view = useView()

const hasGitLab = computed(() => configured.value.some(i => i.id === 'gitlab'))
const hasYouTrack = computed(() => configured.value.some(i => i.id === 'youtrack'))
const issueId = computed(() => active.value && hasYouTrack.value ? links.value[active.value.wt.path] : undefined)

const project = ref<string>()
const mr = ref<MergeRequest>()
const loading = ref(false)
const error = ref<string>()

const title = ref('')
const target = ref('')
const creating = ref(false)
// Форму заполняем один раз на worktree — перечитывание по фокусу не затирает набранное
let prefilledFor: string | undefined

async function prefill(path: string, p: string) {
  prefilledFor = path
  title.value = active.value?.wt.branch ?? ''
  target.value = await defaultBranch(p)
  if (issueId.value) {
    const id = issueId.value
    title.value = await fetchIssue(id).then(i => `${id}: ${i.summary}`, () => `${id}: ${title.value}`)
  }
}

async function load() {
  const wt = active.value?.wt
  if (!wt || !hasGitLab.value) return
  loading.value = true
  try {
    const p = await projectFor(wt.path)
    const found = p && wt.branch ? await mrFor(p, wt.branch) : undefined
    // Пока ждали GitLab, пользователь переключил worktree
    if (wt.path !== active.value?.wt.path) return
    project.value = p
    mr.value = found
    error.value = undefined
    if (p && !found && prefilledFor !== wt.path) await prefill(wt.path, p)
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    loading.value = false
  }
}

// Сброс до загрузки — иначе мелькнёт MR предыдущего worktree
watch(() => active.value?.wt.path, () => {
  mr.value = undefined
  project.value = undefined
  error.value = undefined
})
watch([() => active.value?.wt.path, () => active.value && statusIn(active.value.wt.path), hasGitLab], load, { immediate: true })
useEventListener(window, 'focus', load)
// Пока пайплайн идёт — подтягиваем его статус
const PIPELINE_RUNNING = ['created', 'waiting_for_resource', 'preparing', 'pending', 'running']
useIntervalFn(() => PIPELINE_RUNNING.includes(mr.value?.pipeline?.status ?? '') && load(), 15_000)

const PIPELINE_LABEL: Record<string, string> = {
  success: 'Пайплайн прошёл',
  failed: 'Пайплайн упал',
  canceled: 'Пайплайн отменён',
  skipped: 'Пайплайн пропущен',
  manual: 'Ждёт ручного запуска',
}

async function create() {
  const wt = active.value?.wt
  if (!wt?.branch || !project.value) return
  creating.value = true
  try {
    // GitLab создаёт MR только из ветки, которая уже есть на сервере
    const status = await invoke<{ ahead: number | null }>('git_status', { path: wt.path })
    if (status.ahead !== 0) await invoke('git_push', { path: wt.path })
    const id = issueId.value
    const created = await createMR(project.value, {
      source: wt.branch,
      target: target.value.trim(),
      title: title.value.trim(),
      description: id ? `Задача: ${issueUrl(id)}` : '',
    })
    mr.value = created
    toast.success(`MR !${created.iid} создан`)
    if (id) await comment(id, `Merge request: ${created.url}`).catch(e => toast.error(`Комментарий в YouTrack не добавлен: ${e}`))
  }
  catch (e) {
    toast.error(String(e))
  }
  finally {
    creating.value = false
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex h-8 shrink-0 items-center gap-2 text-xs text-muted-foreground">
      <span v-if="project" class="truncate font-mono">{{ project }}</span>
      <Button variant="ghost" size="icon-xs" class="ml-auto" :disabled="loading || !active || !hasGitLab" @click="load">
        <Spinner v-if="loading" />
        <RefreshCwIcon v-else />
        <span class="sr-only">Обновить</span>
      </Button>
    </div>

    <Empty v-if="!active || !hasGitLab || error || !project || !active.wt.branch" class="flex-1">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <GitMergeIcon />
        </EmptyMedia>
        <EmptyTitle>
          {{ !active ? 'Worktree не выбран'
            : !hasGitLab ? 'GitLab не подключён'
              : error ? 'Не удалось получить MR'
                : loading ? 'Загружаю…'
                  : !project ? 'Remote не на GitLab'
                    : 'Detached HEAD' }}
        </EmptyTitle>
        <EmptyDescription>
          {{ !active ? 'Выберите worktree в сайдбаре'
            : !hasGitLab ? 'Укажите адрес и токен GitLab в настройках'
              : error ?? (loading ? '' : !project ? `Remote репозитория указывает не на ${stored.gitlab?.url}` : 'MR создаётся из ветки') }}
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent v-if="active && !hasGitLab">
        <Button variant="outline" @click="view = 'settings'">
          Открыть настройки
        </Button>
      </EmptyContent>
    </Empty>

    <div v-else-if="mr" class="flex flex-col gap-3">
      <div class="flex items-center gap-2 text-xs text-muted-foreground">
        <span class="font-mono">!{{ mr.iid }}</span>
        <Badge v-if="mr.draft" variant="outline">Draft</Badge>
        <span class="ml-auto">{{ ago(mr.updated) }}</span>
      </div>
      <h3 class="text-sm font-medium select-text">
        {{ mr.title }}
      </h3>
      <span class="font-mono text-xs text-muted-foreground">{{ mr.source }} → {{ mr.target }}</span>
      <Button v-if="mr.pipeline" variant="ghost" class="justify-start" @click="openExternal(mr.pipeline.url)">
        <CircleCheckIcon v-if="mr.pipeline.status === 'success'" data-icon="inline-start" class="text-success" />
        <CircleXIcon v-else-if="mr.pipeline.status === 'failed'" data-icon="inline-start" class="text-destructive" />
        <Spinner v-else-if="PIPELINE_RUNNING.includes(mr.pipeline.status)" data-icon="inline-start" />
        <CircleDashedIcon v-else data-icon="inline-start" />
        {{ PIPELINE_LABEL[mr.pipeline.status] ?? 'Пайплайн идёт' }}
      </Button>
      <Button variant="outline" @click="openExternal(mr.url)">
        <ExternalLinkIcon data-icon="inline-start" />
        Открыть в GitLab
      </Button>
    </div>

    <form v-else class="flex flex-col gap-4" @submit.prevent="create">
      <FieldGroup>
        <Field>
          <FieldLabel for="mr-title">Заголовок</FieldLabel>
          <Input id="mr-title" v-model="title" />
          <FieldDescription v-if="issueId">Из задачи {{ issueId }}; в задачу уйдёт комментарий со ссылкой на MR</FieldDescription>
        </Field>
        <Field>
          <FieldLabel for="mr-target">Целевая ветка</FieldLabel>
          <Input id="mr-target" v-model="target" class="font-mono" />
          <FieldDescription>Из {{ active.wt.branch }}; неопубликованная ветка будет запушена</FieldDescription>
        </Field>
      </FieldGroup>
      <Button type="submit" :disabled="creating || !title.trim() || !target.trim()">
        <Spinner v-if="creating" data-icon="inline-start" />
        <GitMergeIcon v-else data-icon="inline-start" />
        Создать MR
      </Button>
    </form>
  </div>
</template>
