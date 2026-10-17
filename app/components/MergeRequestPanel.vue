<script setup lang="ts">
import type { MergeRequest } from '@/composables/useGitLab'
import { CircleCheckIcon, CircleDashedIcon, CircleXIcon, ExternalLinkIcon, GitMergeIcon, RefreshCwIcon, WrenchIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, useIntervalFn } from '@vueuse/core'
import { toast } from 'vue-sonner'

const { active } = useProjects()
const { configured, option } = useIntegrations()
const { statusIn, sendToAgent } = useTerminals()
// PR GitHub и MR GitLab — одна панель: хостинг определяется по remote
const { remote, forge, api, terms, loadRemote } = useForge()
const { links, issue: fetchIssue, comment, setState, issueUrl } = useYouTrack()
const { openSettings } = useSettings()

const hasYouTrack = computed(() => configured.value.some(i => i.id === 'youtrack'))
const issueId = computed(() => active.value && hasYouTrack.value ? links.value[active.value.wt.path] : undefined)

const mr = ref<MergeRequest>()
// Задачи, уже переведённые после слияния: повторно не трогаем, даже если статус потом сменили руками
const movedOnMerge = persisted<string[]>('merged-issues', [])
const loading = ref(false)
const error = ref<string>()
// worktree, для которого PR/MR уже запрошен: до ответа хостинга форму не показываем
const loadedFor = ref<string>()

const title = ref('')
const target = ref('')
const creating = ref(false)
// Форму заполняем один раз на worktree — перечитывание по фокусу не затирает набранное
let prefilledFor: string | undefined

async function prefill(path: string, defaultBranch: Promise<string>) {
  prefilledFor = path
  title.value = active.value?.wt.branch ?? ''
  target.value = await defaultBranch
  if (issueId.value) {
    const id = issueId.value
    title.value = await fetchIssue(id).then(i => `${id}: ${i.summary}`, () => `${id}: ${title.value}`)
  }
}

async function load() {
  const wt = active.value?.wt
  if (!wt?.git) return
  loading.value = true
  try {
    // remote — заново при каждой загрузке: его могли добавить или сменить из терминала, не уводя фокус с окна
    await loadRemote()
    const f = forge.value
    const a = api.value
    if (!f || !a || wt.path !== active.value?.wt.path) return
    const found = wt.branch ? await a.mrFor(f.path, wt.branch) : undefined
    // Пока ждали хостинг, пользователь переключил worktree
    if (wt.path !== active.value?.wt.path) return
    mr.value = found
    error.value = undefined
    loadedFor.value = wt.path
    if (!found && prefilledFor !== wt.path) await prefill(wt.path, a.defaultBranch(f.path))
    const id = issueId.value
    if (found?.state === 'merged' && id && option('youtrack', 'doneState') && !movedOnMerge.value.includes(id)) {
      movedOnMerge.value.push(id)
      await moveIssue(id, 'doneState')
    }
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
  error.value = undefined
  loadedFor.value = undefined
})
watch([() => active.value?.wt.path, () => active.value && statusIn(active.value.wt.path), () => api.value && forge.value?.url], load, { immediate: true })
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

// Почему показать нечего; undefined — показываем PR/MR или форму его создания
const empty = computed(() => {
  const a = active.value
  const f = forge.value
  if (!a) return { title: 'Worktree не выбран', description: 'Выберите worktree в сайдбаре' }
  if (!a.wt.git) return { title: 'Папка без git', description: `${terms.value.mr} создаётся из ветки git-репозитория` }
  if (remote.value === undefined) return { title: 'Загружаю…', description: '' }
  if (!f) return { title: 'Нет remote', description: 'Добавьте его: git remote add origin <url>' }
  if (!api.value) {
    return f.kind === 'github' || f.kind === 'gitlab'
      ? { title: `${f.name} не подключён`, description: `Укажите адрес и токен ${f.name} в настройках`, settings: true }
      : { title: `${f.name} не поддерживается`, description: 'PR и MR работают с GitHub и GitLab; свой GitLab подключается в настройках', settings: true }
  }
  if (error.value) return { title: `Не удалось получить ${terms.value.mr}`, description: error.value }
  if (loadedFor.value !== a.wt.path) return { title: 'Загружаю…', description: '' }
  if (!a.wt.branch) return { title: 'Detached HEAD', description: `${terms.value.mr} создаётся из ветки` }
})

// Статус задачи из настроек YouTrack; пустой — не меняем
async function moveIssue(id: string, key: 'reviewState' | 'doneState') {
  const state = option('youtrack', key)
  if (!state) return
  try {
    const issue = await fetchIssue(id)
    if (!issue.stateField || issue.state === state) return
    await setState(issue, state)
    toast.success(`${id} → ${state}`)
  }
  catch (e) {
    toast.error(`Статус ${id} не изменён: ${e}`)
  }
}

const fixing = ref(false)
// Логи упавших job'ов — агенту этого worktree: пусть разберётся и починит
async function fixPipeline() {
  const wt = active.value?.wt
  const current = mr.value
  const f = forge.value
  if (!wt || !current?.pipeline || !f || !api.value) return
  fixing.value = true
  try {
    const logs = await api.value.failedJobLogs(f.path, current.pipeline.id)
    const text = [
      `Пайплайн ${terms.value.mr} ${terms.value.ref}${current.iid} упал: ${current.pipeline.url}`,
      logs || 'Упавших job\'ов в пайплайне не нашлось — посмотри его по ссылке.',
      'Разберись в причине и почини.',
    ].join('\n\n')
    if (!await sendToAgent(wt.path, `pipeline-${current.iid}`, text))
      toast.error('Нет агента, который принимает промпт: добавьте Claude Code или Codex в настройках')
  }
  catch (e) {
    toast.error(String(e))
  }
  finally {
    fixing.value = false
  }
}

async function create() {
  const wt = active.value?.wt
  const f = forge.value
  if (!wt?.branch || !f || !api.value) return
  const { mr: term, mrName, ref } = terms.value
  creating.value = true
  try {
    // GitLab и GitHub создают MR/PR только из ветки, которая уже есть на сервере
    const status = await invoke<{ ahead: number | null }>('git_status', { path: wt.path })
    if (status.ahead !== 0) await invoke('git_push', { path: wt.path })
    const id = issueId.value
    const created = await api.value.createMR(f.path, {
      source: wt.branch,
      target: target.value.trim(),
      title: title.value.trim(),
      description: id ? `Задача: ${issueUrl(id)}` : '',
    })
    mr.value = created
    toast.success(`${term} ${ref}${created.iid} создан`)
    if (id) {
      await comment(id, `${mrName}: ${created.url}`).catch(e => toast.error(`Комментарий в YouTrack не добавлен: ${e}`))
      await moveIssue(id, 'reviewState')
    }
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
      <span v-if="api" class="truncate font-mono">{{ forge?.path }}</span>
      <Button variant="ghost" size="icon-xs" class="ml-auto" :disabled="loading || !active?.wt.git" @click="load">
        <Spinner v-if="loading" />
        <RefreshCwIcon v-else />
        <span class="sr-only">Обновить</span>
      </Button>
    </div>

    <Empty v-if="empty" class="flex-1">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <GitMergeIcon />
        </EmptyMedia>
        <EmptyTitle>{{ empty.title }}</EmptyTitle>
        <EmptyDescription>{{ empty.description }}</EmptyDescription>
      </EmptyHeader>
      <EmptyContent v-if="empty.settings">
        <Button variant="outline" @click="openSettings('integrations')">
          Открыть настройки
        </Button>
      </EmptyContent>
    </Empty>

    <div v-else-if="mr" class="flex flex-col gap-3">
      <div class="flex items-center gap-2 text-xs text-muted-foreground">
        <span class="font-mono">{{ terms.ref }}{{ mr.iid }}</span>
        <Badge v-if="mr.state === 'merged'" variant="secondary">Влит</Badge>
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
      <Button v-if="mr.state === 'opened' && mr.pipeline?.status === 'failed'" variant="outline" :disabled="fixing" @click="fixPipeline">
        <Spinner v-if="fixing" data-icon="inline-start" />
        <WrenchIcon v-else data-icon="inline-start" />
        Отправить лог агенту
      </Button>
      <Button variant="outline" @click="openExternal(mr.url)">
        <ExternalLinkIcon data-icon="inline-start" />
        Открыть в {{ forge?.name }}
      </Button>
    </div>

    <form v-else class="flex flex-col gap-4" @submit.prevent="create">
      <FieldGroup>
        <Field>
          <FieldLabel for="mr-title">Заголовок</FieldLabel>
          <Input id="mr-title" v-model="title" />
          <FieldDescription v-if="issueId">
            Из задачи {{ issueId }}; в задачу уйдёт комментарий со ссылкой на {{ terms.mr }}{{ option('youtrack', 'reviewState') && `, статус — ${option('youtrack', 'reviewState')}` }}
          </FieldDescription>
        </Field>
        <Field>
          <FieldLabel for="mr-target">Целевая ветка</FieldLabel>
          <Input id="mr-target" v-model="target" class="font-mono" />
          <FieldDescription>Из {{ active?.wt.branch }}; неопубликованная ветка будет запушена</FieldDescription>
        </Field>
      </FieldGroup>
      <Button type="submit" :disabled="creating || !title.trim() || !target.trim()">
        <Spinner v-if="creating" data-icon="inline-start" />
        <GitMergeIcon v-else data-icon="inline-start" />
        Создать {{ terms.mr }}
      </Button>
    </form>
  </div>
</template>
