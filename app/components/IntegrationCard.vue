<script setup lang="ts">
import type { IntegrationDef } from '@/composables/useIntegrations'
import { toast } from 'vue-sonner'

const props = defineProps<{ def: IntegrationDef }>()
const { stored, configured, option, save, remove, whoami } = useIntegrations()

const current = computed(() => stored.value[props.def.id])
const isOn = computed(() => configured.value.some(i => i.id === props.def.id))

const url = ref('')
// Токен вводится и сразу уходит в Keychain; сохранённый в форму не возвращается
const token = ref('')
const values = ref<Record<string, string>>({})
watch(current, (c) => {
  url.value = c?.url ?? ''
  values.value = Object.fromEntries((props.def.options ?? []).map(o => [o.key, option(props.def.id, o.key)]))
}, { immediate: true })

const busy = ref<'save' | 'remove'>()
const me = ref<string>()
const error = ref<string>()
const formId = `integration-${props.def.id}`

async function onSave() {
  busy.value = 'save'
  error.value = undefined
  try {
    await save(props.def.id, url.value, token.value, values.value)
    token.value = ''
    me.value = await whoami(props.def.id)
    toast.success(`${props.def.name}: подключено как ${me.value}`)
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    busy.value = undefined
  }
}

async function onRemove() {
  busy.value = 'remove'
  try {
    await remove(props.def.id)
    me.value = undefined
  }
  catch (e) {
    toast.error(String(e))
  }
  finally {
    busy.value = undefined
  }
}
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle class="flex items-center gap-2">
        <component :is="def.icon" class="size-4" />
        {{ def.name }}
      </CardTitle>
      <CardDescription>
        {{ me ? `Подключено как ${me}` : isOn ? 'Подключено' : 'Не настроено' }}
      </CardDescription>
      <CardAction>
        <Badge :variant="isOn ? 'secondary' : 'outline'">
          {{ isOn ? 'Активна' : 'Выключена' }}
        </Badge>
      </CardAction>
    </CardHeader>
    <CardContent>
      <form :id="formId" @submit.prevent="onSave">
        <FieldGroup>
          <Field>
            <FieldLabel :for="`${formId}-url`">Адрес</FieldLabel>
            <Input :id="`${formId}-url`" v-model="url" type="url" :placeholder="def.urlPlaceholder" />
          </Field>
          <Field :data-invalid="!!error || undefined">
            <FieldLabel :for="`${formId}-token`">Токен</FieldLabel>
            <Input
              :id="`${formId}-token`"
              v-model="token"
              type="password"
              autocomplete="off"
              :placeholder="current?.hasToken ? 'Сохранён в Keychain — оставьте пустым, чтобы не менять' : 'Вставьте токен'"
              :aria-invalid="!!error || undefined"
            />
            <FieldDescription>{{ def.tokenHint }}</FieldDescription>
            <FieldError :errors="[error]" />
          </Field>
          <Field v-for="o in def.options" :key="o.key">
            <FieldLabel :for="`${formId}-${o.key}`">{{ o.label }}</FieldLabel>
            <Input :id="`${formId}-${o.key}`" v-model="values[o.key]" :placeholder="o.default" class="font-mono" />
          </Field>
        </FieldGroup>
      </form>
    </CardContent>
    <CardFooter class="gap-2">
      <Button type="submit" :form="formId" :disabled="!!busy || !url.trim() || (!token && !current?.hasToken)">
        <Spinner v-if="busy === 'save'" data-icon="inline-start" />
        Сохранить и проверить
      </Button>
      <Button v-if="current" variant="ghost" :disabled="!!busy" @click="onRemove">
        Отключить
      </Button>
    </CardFooter>
  </Card>
</template>
