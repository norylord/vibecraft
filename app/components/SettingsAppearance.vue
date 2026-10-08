<script setup lang="ts">
import { MonitorIcon, MoonIcon, SunIcon } from '@lucide/vue'

const { settings, shortcut } = useSettings()

const THEMES = [
  { value: 'dark', label: 'Тёмная', icon: MoonIcon },
  { value: 'light', label: 'Светлая', icon: SunIcon },
  { value: 'system', label: 'Как в системе', icon: MonitorIcon },
] as const
const ZOOMS = [0.8, 0.9, 1, 1.1, 1.25, 1.5]
const FONT_SIZES = [11, 12, 13, 14, 15, 16]

// ToggleGroup отдаёт строки и пустое значение при повторном клике — числа и «нельзя снять выбор» держим здесь
const theme = computed({
  get: () => settings.value.theme,
  set: v => v && (settings.value.theme = v),
})
const zoom = computed({
  get: () => String(settings.value.zoom),
  set: v => v && (settings.value.zoom = Number(v)),
})
const fontSize = computed({
  get: () => String(settings.value.terminalFontSize),
  set: v => v && (settings.value.terminalFontSize = Number(v)),
})
</script>

<template>
  <FieldGroup>
    <Field>
      <FieldLabel>Тема</FieldLabel>
      <ToggleGroup v-model="theme" type="single" variant="outline" class="justify-start">
        <ToggleGroupItem v-for="t in THEMES" :key="t.value" :value="t.value" class="px-3">
          <component :is="t.icon" />
          {{ t.label }}
        </ToggleGroupItem>
      </ToggleGroup>
      <FieldDescription>Вместе с окном меняется и размытие под сайдбаром</FieldDescription>
    </Field>
    <FieldSeparator />
    <Field>
      <FieldLabel>Масштаб интерфейса</FieldLabel>
      <ToggleGroup v-model="zoom" type="single" variant="outline" class="justify-start">
        <ToggleGroupItem v-for="z in ZOOMS" :key="z" :value="String(z)" class="px-3">
          {{ Math.round(z * 100) }}%
        </ToggleGroupItem>
      </ToggleGroup>
      <FieldDescription>
        Шаг 10%: {{ shortcut('zoomIn') }} и {{ shortcut('zoomOut') }}, сброс — {{ shortcut('zoomReset') }}. Сейчас {{ Math.round(settings.zoom * 100) }}%
      </FieldDescription>
    </Field>
    <FieldSeparator />
    <Field>
      <FieldLabel>Шрифт терминала</FieldLabel>
      <ToggleGroup v-model="fontSize" type="single" variant="outline" class="justify-start">
        <ToggleGroupItem v-for="size in FONT_SIZES" :key="size" :value="String(size)" class="px-3 font-mono">
          {{ size }}
        </ToggleGroupItem>
      </ToggleGroup>
      <FieldDescription>Применяется к открытым терминалам сразу — агент перерисуется под новую ширину</FieldDescription>
    </Field>
  </FieldGroup>
</template>
