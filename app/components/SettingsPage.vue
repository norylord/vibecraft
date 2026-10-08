<script setup lang="ts">
import { INTEGRATIONS } from '@/composables/useIntegrations'
</script>

<template>
  <SidebarInset class="min-w-0 overflow-hidden animate-in fade-in duration-200">
    <AppHeader>Настройки</AppHeader>
    <div class="min-h-0 flex-1 overflow-auto">
      <div class="mx-auto flex max-w-2xl flex-col gap-4 p-6">
        <div class="flex flex-col gap-1">
          <h1 class="font-heading text-base font-medium">
            Интеграции
          </h1>
          <p class="text-sm text-muted-foreground">
            Токены хранятся в Keychain macOS и не покидают Rust-часть приложения.
            Подключённые интеграции появляются ярлыками в сайдбаре.
          </p>
        </div>
        <IntegrationCard v-for="def in INTEGRATIONS" :key="def.id" :def="def" />
      </div>
    </div>
  </SidebarInset>
</template>
