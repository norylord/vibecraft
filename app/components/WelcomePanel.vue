<script setup lang="ts">
import { BotIcon, CircleCheckIcon, FolderGit2Icon, PlugIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import logoUrl from '~~/src-tauri/app-icon.svg?url'
import { cn } from '@/lib/utils'

// Первый запуск: пока нет ни одного проекта, вместо пустого workspace — чек-лист
const emit = defineEmits<{ agents: [] }>()
const { projects, addProject } = useProjects()
const { runnable: agents } = useAgents()
const { configured } = useIntegrations()
const view = useView()

// Установлен ли CLI агента — по PATH из login-shell, как в терминале
const installed = ref<boolean[]>([])
watch(agents, async (list) => {
  installed.value = await invoke<boolean[]>('installed', { programs: list.map(a => agentProgram(a.command) ?? '') })
}, { immediate: true, deep: true })

const steps = computed(() => [
  {
    icon: FolderGit2Icon,
    title: 'Добавьте проект',
    description: 'Git-репозиторий, в worktree которого будут работать агенты',
    done: projects.value.length > 0,
    action: { label: 'Добавить проект', run: addProject },
  },
  {
    icon: BotIcon,
    title: 'Проверьте агентов',
    description: agents.value.map((a, i) => `${a.name} ${installed.value[i] ? '✓' : '— не найден'}`).join(' · '),
    done: installed.value.some(Boolean),
    action: { label: 'Настроить', run: () => emit('agents') },
  },
  {
    icon: PlugIcon,
    title: 'Подключите интеграции',
    description: 'Необязательно: задачи YouTrack → worktree с агентом, MR и пайплайны GitLab',
    done: configured.value.length > 0,
    action: { label: 'Настройки', run: () => (view.value = 'settings') },
  },
])
</script>

<template>
  <div class="flex h-full items-center justify-center overflow-auto p-6">
    <div class="flex w-full max-w-md flex-col gap-6 animate-in fade-in slide-in-from-bottom-2 duration-500">
      <div class="flex flex-col items-center gap-3 text-center">
        <img :src="logoUrl" alt="" class="size-20 drop-shadow-lg">
        <h1 class="font-heading text-lg font-medium">
          Добро пожаловать в Diogen
        </h1>
        <p class="text-sm text-muted-foreground">
          Агенты в изолированных worktree, задачи из YouTrack и MR в GitLab — в одном окне
        </p>
      </div>
      <Card class="gap-0 py-0">
        <template v-for="(step, i) in steps" :key="step.title">
          <Separator v-if="i" />
          <div class="flex items-center gap-3 p-4 animate-in fade-in fill-mode-backwards" :style="{ animationDelay: `${150 + i * 80}ms` }">
            <CircleCheckIcon v-if="step.done" class="size-5 shrink-0 text-success" />
            <component :is="step.icon" v-else class="size-5 shrink-0 text-muted-foreground" />
            <div class="flex min-w-0 flex-1 flex-col gap-0.5">
              <span :class="cn('text-sm font-medium', step.done && 'text-muted-foreground')">{{ step.title }}</span>
              <span class="text-xs text-muted-foreground">{{ step.description }}</span>
            </div>
            <Button size="sm" :variant="step.done ? 'ghost' : 'outline'" @click="step.action.run()">
              {{ step.action.label }}
            </Button>
          </div>
        </template>
      </Card>
    </div>
  </div>
</template>
