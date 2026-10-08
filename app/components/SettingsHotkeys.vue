<script setup lang="ts">
import type { ActionId } from '@/utils/hotkeys'
import { useEventListener } from '@vueuse/core'

const { settings, binding, shortcut } = useSettings()

const recording = ref<ActionId>()
const error = ref<string>()

function record(id: ActionId) {
  recording.value = id
  error.value = undefined
}

// capture + stopPropagation: пока ждём сочетание, Workspace не должен его выполнить
useEventListener('keydown', (e: KeyboardEvent) => {
  if (!recording.value) return
  e.preventDefault()
  e.stopPropagation()
  if (e.key === 'Escape') return void (recording.value = undefined)
  const combo = comboOf(e)
  if (!combo) return
  const id = recording.value
  if (RESERVED.includes(combo)) return void (error.value = `${formatCombo(combo)} — системное сочетание macOS`)
  const taken = ACTIONS.find(a => a.id !== id && binding(a.id) === combo)
  if (taken) return void (error.value = `${formatCombo(combo)} уже занято: «${taken.label}»`)
  settings.value.hotkeys = { ...settings.value.hotkeys, [id]: combo }
  recording.value = undefined
  error.value = undefined
}, { capture: true })

function reset(id?: ActionId) {
  const { [id as ActionId]: _, ...rest } = settings.value.hotkeys
  settings.value.hotkeys = id ? rest : {}
}

const isCustom = (id: ActionId) => id in settings.value.hotkeys
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex items-center justify-between gap-2">
      <p class="text-xs text-muted-foreground">
        Нажмите «Изменить» и новое сочетание. Esc — отмена. Работают в любой раскладке.
      </p>
      <Button variant="ghost" size="sm" :disabled="!Object.keys(settings.hotkeys).length" @click="reset()">
        Сбросить все
      </Button>
    </div>
    <Card class="gap-0 py-0">
      <template v-for="(action, i) in ACTIONS" :key="action.id">
        <Separator v-if="i" />
        <div class="flex items-center gap-3 px-4 py-2.5">
          <span class="flex-1 text-sm">{{ action.label }}</span>
          <Kbd v-if="recording === action.id" class="animate-pulse">Нажмите сочетание…</Kbd>
          <Kbd v-else>{{ shortcut(action.id) }}</Kbd>
          <Button variant="ghost" size="sm" @click="recording === action.id ? (recording = undefined) : record(action.id)">
            {{ recording === action.id ? 'Отмена' : 'Изменить' }}
          </Button>
          <Button variant="ghost" size="sm" :class="!isCustom(action.id) && 'invisible'" @click="reset(action.id)">
            Сбросить
          </Button>
        </div>
      </template>
    </Card>
    <Alert v-if="error" variant="destructive">
      <AlertDescription>{{ error }}</AlertDescription>
    </Alert>
  </div>
</template>
