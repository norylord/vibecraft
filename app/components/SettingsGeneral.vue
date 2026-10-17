<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { computedAsync } from '@vueuse/core'

const { settings } = useSettings()
const { checking, checkUpdate } = useUpdater()
const version = computedAsync(getVersion)
</script>

<template>
  <FieldGroup>
    <Field>
      <FieldLabel for="settings-editor">Редактор</FieldLabel>
      <Input id="settings-editor" v-model="settings.editor" placeholder="WebStorm" />
      <FieldDescription>Приложение для «Открыть в …»: имя из /Applications — WebStorm, Visual Studio Code, Cursor, Zed</FieldDescription>
    </Field>
    <FieldSeparator />
    <Field orientation="horizontal">
      <FieldContent>
        <FieldLabel for="settings-notifications">Системные уведомления</FieldLabel>
        <FieldDescription>Когда агент ждёт ввода или закончил, а окно Vibecraft не в фокусе</FieldDescription>
      </FieldContent>
      <Switch id="settings-notifications" v-model="settings.notifications" />
    </Field>
    <Field orientation="horizontal">
      <FieldContent>
        <FieldLabel for="settings-collapse">Агенту — вся ширина</FieldLabel>
        <FieldDescription>Сворачивать правую панель, когда задача из YouTrack запускает агента</FieldDescription>
      </FieldContent>
      <Switch id="settings-collapse" v-model="settings.collapsePanelOnStart" />
    </Field>
    <FieldSeparator />
    <Field orientation="horizontal">
      <FieldContent>
        <FieldLabel>Обновления</FieldLabel>
        <FieldDescription>Vibecraft {{ version }}. Новые версии проверяются при запуске и раз в 6 часов</FieldDescription>
      </FieldContent>
      <Button variant="outline" :disabled="checking" @click="checkUpdate(true)">
        <Spinner v-if="checking" data-icon="inline-start" />
        Проверить
      </Button>
    </Field>
  </FieldGroup>
</template>
