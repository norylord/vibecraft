<script setup lang="ts">
import type { UsageWindow } from '@/utils/usage'
import { invoke } from '@tauri-apps/api/core'
import { useIntervalFn } from '@vueuse/core'
import { cn } from '@/lib/utils'

// Лимиты подписки умеют отдавать Claude Code и Codex (src-tauri/src/usage.rs)
const SOURCES = {
  claude: { command: 'usage_claude', parse: claudeWindows },
  codex: { command: 'usage_codex', parse: codexWindows },
}
type Program = keyof typeof SOURCES

const { runnable } = useAgents()
// Блок на программу из пресетов, имя — первого пресета с ней
const programs = computed(() => {
  const found = new Map<Program, string>()
  for (const a of runnable.value) {
    const p = agentProgram(a.command)
    if (p && p in SOURCES && !found.has(p as Program)) found.set(p as Program, a.name)
  }
  return [...found]
})

const usage = ref<Partial<Record<Program, { windows: UsageWindow[], error?: string }>>>({})

async function load() {
  await Promise.all(programs.value.map(async ([p]) => {
    try {
      usage.value[p] = { windows: SOURCES[p].parse(await invoke(SOURCES[p].command)) }
    }
    catch (e) {
      usage.value[p] = { windows: [], error: String(e) }
    }
  }))
}
// ponytail: опрос раз в 5 минут — чаще api.anthropic.com отвечает 429
useIntervalFn(load, 5 * 60_000, { immediateCallback: true })
// Ключом, а не массивом: правка имени пресета в настройках не должна дёргать запросы
watch(() => programs.value.map(([p]) => p).join(), load)

const barColor = (percent: number) => percent >= 90 ? 'bg-destructive' : percent >= 70 ? 'bg-warning' : 'bg-primary'
</script>

<template>
  <div v-if="programs.length" class="flex flex-col gap-2 px-2 py-1 text-xs text-muted-foreground">
    <div v-for="[p, name] in programs" :key="p" class="flex flex-col gap-1">
      <span class="font-medium text-sidebar-foreground/70">{{ name }}</span>
      <span v-if="usage[p]?.error" class="truncate" :title="usage[p].error">{{ usage[p].error }}</span>
      <Tooltip v-for="w in usage[p]?.windows" :key="w.label" :disabled="!w.resetsAt">
        <TooltipTrigger as-child>
          <div class="grid grid-cols-[3.5rem_1fr_2.5rem] items-center gap-2">
            <span>{{ w.label }}</span>
            <div class="h-1.5 overflow-hidden rounded-full bg-muted" aria-hidden="true">
              <div :class="cn('h-full rounded-full transition-[width]', barColor(w.percent))" :style="{ width: `${Math.min(w.percent, 100)}%` }" />
            </div>
            <span class="text-right tabular-nums">{{ Math.round(w.percent) }}%</span>
          </div>
        </TooltipTrigger>
        <TooltipContent side="right">
          Сброс {{ w.resetsAt && ago(w.resetsAt) }}
        </TooltipContent>
      </Tooltip>
    </div>
  </div>
</template>
