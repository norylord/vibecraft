<script setup lang="ts">
import type { AgentStatus } from '@/composables/useTerminals'
import { BellRingIcon, CircleCheckIcon } from '@lucide/vue'

defineProps<{ status?: AgentStatus }>()
</script>

<!-- Один корневой svg: размер задают родители (кнопки, табы). Смена статуса пересоздаёт иконку — zoom-in проигрывается заново -->
<template>
  <!-- Без animate-in: он перебил бы animate-spin крутилки -->
  <Spinner v-if="status === 'working'" aria-label="Агент работает" />
  <BellRingIcon v-else-if="status === 'waiting'" class="text-warning animate-in zoom-in-50 duration-200" aria-label="Агент ждёт ввода" />
  <CircleCheckIcon v-else-if="status === 'done'" class="text-success animate-in zoom-in-50 duration-200" aria-label="Агент закончил" />
  <slot v-else />
</template>
