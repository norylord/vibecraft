<script setup lang="ts">
import type { Issue } from '@/composables/useYouTrack'
import { PlayIcon } from '@lucide/vue'
import { toast } from 'vue-sonner'

const issue = defineModel<Issue | undefined>('issue')
const { projects } = useProjects()
const { runnable: agents } = useAgents()
const { start, repoFor } = useYouTrack()
const view = useView()

const repo = ref('')
const agentName = ref('')
const branch = ref('')
const busy = ref(false)
const error = ref<string>()

watch(issue, (i) => {
  if (!i) return
  repo.value = repoFor.value[i.project] ?? projects.value[0] ?? ''
  agentName.value = agents.value[0]?.name ?? ''
  branch.value = branchName(i.id, i.summary)
  error.value = undefined
})

async function submit() {
  const agent = agents.value.find(a => a.name === agentName.value)
  if (!issue.value || !agent || !repo.value) return
  busy.value = true
  try {
    const result = await start(issue.value, repo.value, branch.value.trim(), agent)
    if (result === 'clipboard') toast.info('Промпт задачи в буфере обмена — вставьте его в агента')
    issue.value = undefined
    view.value = 'workspace'
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog :open="!!issue" @update:open="open => !open && (issue = undefined)">
    <DialogContent class="sm:max-w-md">
      <form class="flex flex-col gap-4" @submit.prevent="submit">
        <DialogHeader>
          <DialogTitle>Начать {{ issue?.id }}</DialogTitle>
          <DialogDescription class="line-clamp-2">
            {{ issue?.summary }}
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field :data-invalid="!projects.length || undefined">
            <FieldLabel for="start-repo">Репозиторий</FieldLabel>
            <Select v-model="repo">
              <SelectTrigger id="start-repo" class="w-full">
                <SelectValue placeholder="Выберите проект" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem v-for="p in projects" :key="p" :value="p">
                    {{ baseName(p) }}
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
            <FieldError v-if="!projects.length" :errors="['Сначала добавьте проект в сайдбаре']" />
          </Field>
          <Field>
            <FieldLabel for="start-agent">Агент</FieldLabel>
            <Select v-model="agentName">
              <SelectTrigger id="start-agent" class="w-full">
                <SelectValue placeholder="Выберите агента" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem v-for="a in agents" :key="a.name" :value="a.name">
                    {{ a.name }}
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
          <Field :data-invalid="!!error || undefined">
            <FieldLabel for="start-branch">Ветка</FieldLabel>
            <Input id="start-branch" v-model="branch" class="font-mono" :aria-invalid="!!error || undefined" />
            <FieldDescription>Если ветка или worktree уже есть — переиспользуем</FieldDescription>
            <FieldError :errors="[error]" />
          </Field>
        </FieldGroup>
        <DialogFooter>
          <Button type="submit" :disabled="busy || !repo || !agentName || !branch.trim()">
            <Spinner v-if="busy" data-icon="inline-start" />
            <PlayIcon v-else data-icon="inline-start" />
            Начать
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
