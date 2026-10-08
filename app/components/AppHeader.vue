<script setup lang="ts">
import { useSidebar } from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'

const { state } = useSidebar()
</script>

<template>
  <!-- pl-20 при скрытом сайдбаре: не залезать под «светофор» macOS -->
  <header
    data-tauri-drag-region
    :class="cn('flex h-10 shrink-0 items-center gap-2 px-2 transition-[padding]', state === 'collapsed' && 'pl-20')"
  >
    <SidebarTrigger />
    <span data-tauri-drag-region class="truncate text-xs text-muted-foreground">
      <slot />
    </span>
    <div class="ml-auto flex items-center gap-1">
      <slot name="actions" />
    </div>
  </header>
  <Separator />
</template>
