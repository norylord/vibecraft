<script setup lang="ts">
const { settings } = useSettings()
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
        <FieldDescription>Когда агент ждёт ввода или закончил, а окно Diogen не в фокусе</FieldDescription>
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
  </FieldGroup>
</template>
