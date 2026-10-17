<script setup lang="ts">
const { update, open, progress, install } = useUpdater()

// Список изменений из CI: «### Раздел» и «- пункт»
const lines = computed(() => (update.value?.body ?? '').split('\n').map(l => l.trim()).filter(Boolean))
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>Vibecraft {{ update?.version }}</DialogTitle>
        <DialogDescription>
          Сейчас установлена {{ update?.currentVersion }}. Терминалы и агенты переживут перезапуск
        </DialogDescription>
      </DialogHeader>
      <div class="flex max-h-80 flex-col gap-1 overflow-auto text-sm">
        <template v-for="(line, i) in lines" :key="i">
          <p v-if="line.startsWith('- ')" class="flex gap-2 text-muted-foreground">
            <span>•</span>
            <span class="first-letter:uppercase">{{ line.slice(2) }}</span>
          </p>
          <h3 v-else class="mt-2 font-medium first:mt-0">
            {{ line.replace(/^#+\s*/, '') }}
          </h3>
        </template>
      </div>
      <DialogFooter>
        <Button variant="outline" :disabled="progress !== undefined" @click="open = false">
          Позже
        </Button>
        <Button :disabled="progress !== undefined" @click="install">
          <Spinner v-if="progress !== undefined" data-icon="inline-start" />
          {{ progress === undefined ? 'Обновить и перезапустить' : `Загрузка ${Math.round(progress * 100)}%` }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
