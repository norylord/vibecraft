<script setup lang="ts">
import type { AgentStatus } from '@/composables/useTerminals'
import { BellRingIcon, CircleCheckIcon } from '@lucide/vue'

defineProps<{ status?: AgentStatus }>()
</script>

<!-- Один корневой svg: родительские компоненты (кнопки, табы) сами задают ему размер -->
<template>
  <Spinner v-if="status === 'working'" aria-label="Агент работает" />
  <BellRingIcon v-else-if="status === 'waiting'" class="text-warning" aria-label="Агент ждёт ввода" />
  <CircleCheckIcon v-else-if="status === 'done'" class="text-success" aria-label="Агент закончил" />
  <slot v-else />
</template>
