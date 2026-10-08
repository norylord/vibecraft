<script setup lang="ts">
import { PlusIcon, Trash2Icon } from '@lucide/vue'

const open = defineModel<boolean>('open', { required: true })
const { agents } = useAgents()
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>Агенты</DialogTitle>
        <DialogDescription>
          Команда печатается в shell worktree — флаги и переменные пишутся прямо в ней:
          <code class="font-mono">FOO=1 claude --model opus</code>
        </DialogDescription>
      </DialogHeader>
      <FieldGroup class="gap-3">
        <div v-for="(agent, i) in agents" :key="i" class="flex items-end gap-2">
          <Field class="w-40 shrink-0">
            <FieldLabel :for="`agent-name-${i}`" :class="i > 0 && 'sr-only'">Название</FieldLabel>
            <Input :id="`agent-name-${i}`" v-model="agent.name" placeholder="Claude Code" />
          </Field>
          <Field>
            <FieldLabel :for="`agent-cmd-${i}`" :class="i > 0 && 'sr-only'">Команда</FieldLabel>
            <Input :id="`agent-cmd-${i}`" v-model="agent.command" placeholder="claude" class="font-mono" />
          </Field>
          <Button variant="ghost" size="icon" @click="agents.splice(i, 1)">
            <Trash2Icon />
            <span class="sr-only">Удалить агента</span>
          </Button>
        </div>
      </FieldGroup>
      <DialogFooter>
        <Button variant="outline" @click="agents.push({ name: '', command: '' })">
          <PlusIcon data-icon="inline-start" />
          Добавить агента
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
