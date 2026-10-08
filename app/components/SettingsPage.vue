<script setup lang="ts">
import type { Component } from 'vue'
import type { SettingsTab } from '@/composables/useSettings'
import { BotIcon, KeyboardIcon, PaletteIcon, PlugIcon, SlidersHorizontalIcon } from '@lucide/vue'
import { INTEGRATIONS } from '@/composables/useIntegrations'

const { tab } = useSettings()

const SECTIONS: { id: SettingsTab, label: string, icon: Component, description: string }[] = [
  { id: 'general', label: 'Общие', icon: SlidersHorizontalIcon, description: 'Редактор, уведомления, поведение' },
  { id: 'appearance', label: 'Внешний вид', icon: PaletteIcon, description: 'Тема, масштаб, шрифт терминала' },
  { id: 'hotkeys', label: 'Горячие клавиши', icon: KeyboardIcon, description: 'Сочетания для действий Diogen' },
  { id: 'agents', label: 'Агенты', icon: BotIcon, description: 'CLI-агенты, которых можно запускать в worktree' },
  { id: 'integrations', label: 'Интеграции', icon: PlugIcon, description: 'Токены хранятся в Keychain macOS и не покидают Rust-часть приложения. Подключённые интеграции появляются ярлыками в сайдбаре' },
]
const current = computed(() => SECTIONS.find(s => s.id === tab.value)!)
</script>

<template>
  <SidebarInset class="min-w-0 overflow-hidden animate-in fade-in duration-200">
    <AppHeader>Настройки</AppHeader>
    <Tabs v-model="tab" orientation="vertical" class="min-h-0 flex-1 gap-0">
      <TabsList variant="line" class="w-52 shrink-0 items-stretch border-r p-3">
        <TabsTrigger v-for="s in SECTIONS" :key="s.id" :value="s.id">
          <component :is="s.icon" />
          {{ s.label }}
        </TabsTrigger>
      </TabsList>
      <div class="min-h-0 flex-1 overflow-auto">
        <div class="mx-auto flex max-w-2xl flex-col gap-6 p-6">
          <div class="flex flex-col gap-1">
            <h1 class="font-heading text-base font-medium">
              {{ current.label }}
            </h1>
            <p class="text-sm text-muted-foreground">
              {{ current.description }}
            </p>
          </div>
          <!-- key: при смене раздела контент проявляется заново -->
          <div :key="tab" class="animate-in fade-in duration-200">
            <TabsContent value="general">
              <SettingsGeneral />
            </TabsContent>
            <TabsContent value="appearance">
              <SettingsAppearance />
            </TabsContent>
            <TabsContent value="hotkeys">
              <SettingsHotkeys />
            </TabsContent>
            <TabsContent value="agents">
              <SettingsAgents />
            </TabsContent>
            <TabsContent value="integrations" class="flex flex-col gap-4">
              <IntegrationCard v-for="def in INTEGRATIONS" :key="def.id" :def="def" />
            </TabsContent>
          </div>
        </div>
      </div>
    </Tabs>
  </SidebarInset>
</template>
