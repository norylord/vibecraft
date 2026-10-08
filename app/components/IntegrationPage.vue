<script setup lang="ts">
import type { IntegrationId } from '@/composables/useIntegrations'
import { INTEGRATIONS } from '@/composables/useIntegrations'

const props = defineProps<{ id: IntegrationId }>()
const def = computed(() => INTEGRATIONS.find(i => i.id === props.id)!)
const { whoami } = useIntegrations()

const me = ref<string>()
const error = ref<string>()
watch(() => props.id, async (id) => {
  me.value = undefined
  error.value = undefined
  try {
    me.value = await whoami(id)
  }
  catch (e) {
    error.value = String(e)
  }
}, { immediate: true })
</script>

<!-- Экран интеграции: списки задач YouTrack и MR GitLab — следующие шаги -->
<template>
  <SidebarInset class="min-w-0 overflow-hidden">
    <AppHeader>{{ def.name }}</AppHeader>
    <Empty class="flex-1">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <component :is="def.icon" />
        </EmptyMedia>
        <EmptyTitle>{{ error ? 'Нет подключения' : me ? `Подключено как ${me}` : 'Подключаюсь…' }}</EmptyTitle>
        <EmptyDescription>{{ error ?? 'Здесь появятся задачи и merge requests' }}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  </SidebarInset>
</template>
