<script setup lang="ts">
import { PlusIcon, Trash2Icon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'

const { agents } = useAgents()

// Есть ли CLI в PATH пользователя — подсказка, что агент не запустится
const installed = ref<boolean[]>([])
watch(agents, async (list) => {
  installed.value = await invoke<boolean[]>('installed', { programs: list.map(a => agentProgram(a.command) ?? '') })
}, { immediate: true, deep: true })
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="text-xs text-muted-foreground">
      Команда печатается в shell worktree — флаги и переменные пишутся прямо в ней:
      <code class="font-mono">FOO=1 claude --model opus</code>.
      Для claude и codex Diogen сам добавит хуки статусов.
    </p>
    <FieldGroup class="gap-3">
      <div v-for="(agent, i) in agents" :key="i" class="flex items-end gap-2">
        <Field class="w-40 shrink-0">
          <FieldLabel :for="`agent-name-${i}`" :class="i > 0 && 'sr-only'">Название</FieldLabel>
          <Input :id="`agent-name-${i}`" v-model="agent.name" placeholder="Claude Code" />
        </Field>
        <Field :data-invalid="(agent.command.trim() && installed[i] === false) || undefined">
          <FieldLabel :for="`agent-cmd-${i}`" :class="i > 0 && 'sr-only'">Команда</FieldLabel>
          <Input :id="`agent-cmd-${i}`" v-model="agent.command" placeholder="claude" class="font-mono" />
        </Field>
        <Badge :variant="installed[i] ? 'secondary' : 'outline'" class="mb-1.5 shrink-0">
          {{ installed[i] ? 'найден' : 'нет в PATH' }}
        </Badge>
        <Button variant="ghost" size="icon" @click="agents.splice(i, 1)">
          <Trash2Icon />
          <span class="sr-only">Удалить агента</span>
        </Button>
      </div>
    </FieldGroup>
    <div>
      <Button variant="outline" @click="agents.push({ name: '', command: '' })">
        <PlusIcon data-icon="inline-start" />
        Добавить агента
      </Button>
    </div>
  </div>
</template>
