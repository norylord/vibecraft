<script setup lang="ts">
import type { SshServer } from '@/utils/ssh'
import { open } from '@tauri-apps/plugin-dialog'
import { homeDir } from '@tauri-apps/api/path'

// Копия сервера: правки попадают в список только по «Сохранить»
const server = defineModel<SshServer | undefined>('server')
const { servers, passwords, save, forget, local } = useSsh()

// Пароль вводится и сразу уходит в credentials.json; сохранённый в форму не возвращается
const password = ref('')
const keys = ref<string[]>([])
const busy = ref(false)
const error = ref<string>()
const isNew = computed(() => !servers.value.some(s => s.id === server.value?.id))

watch(server, async (s) => {
  if (!s) return
  password.value = ''
  error.value = undefined
  keys.value = (await local()).keys
})

async function browse() {
  const path = await open({ title: 'Приватный ключ', defaultPath: `${await homeDir()}/.ssh` })
  if (path && server.value) server.value.key = path
}

async function submit() {
  if (!server.value || busy.value) return
  busy.value = true
  error.value = undefined
  try {
    await save({ ...server.value }, password.value)
    server.value = undefined
  }
  catch (e) {
    error.value = String(e)
  }
  finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog :open="!!server" @update:open="o => !o && (server = undefined)">
    <DialogContent class="sm:max-w-md">
      <form v-if="server" class="flex flex-col gap-4" @submit.prevent="submit">
        <DialogHeader>
          <DialogTitle>{{ isNew ? 'Новый сервер' : server.name || server.host }}</DialogTitle>
          <DialogDescription>Подключение откроется вкладкой терминала в текущем worktree</DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field>
            <FieldLabel for="ssh-name">Название</FieldLabel>
            <Input id="ssh-name" v-model="server.name" :placeholder="server.host || 'prod'" />
          </Field>
          <div class="flex gap-2">
            <Field>
              <FieldLabel for="ssh-host">Хост</FieldLabel>
              <Input id="ssh-host" v-model="server.host" required placeholder="example.com" class="font-mono" />
            </Field>
            <Field class="w-24 shrink-0">
              <FieldLabel for="ssh-port">Порт</FieldLabel>
              <Input id="ssh-port" v-model="server.port" inputmode="numeric" pattern="[0-9]{1,5}" placeholder="22" class="font-mono" />
            </Field>
          </div>
          <Field>
            <FieldLabel for="ssh-user">Пользователь</FieldLabel>
            <Input id="ssh-user" v-model="server.user" placeholder="root" class="font-mono" />
          </Field>
          <Field>
            <FieldLabel for="ssh-key">Ключ</FieldLabel>
            <InputGroup>
              <InputGroupInput id="ssh-key" v-model="server.key" list="ssh-keys" placeholder="ssh-agent и ~/.ssh/config" class="font-mono" />
              <InputGroupAddon align="inline-end">
                <InputGroupButton @click="browse">
                  Обзор…
                </InputGroupButton>
              </InputGroupAddon>
            </InputGroup>
            <datalist id="ssh-keys">
              <option v-for="k in keys" :key="k" :value="k" />
            </datalist>
          </Field>
          <Field :data-invalid="!!error || undefined">
            <FieldLabel for="ssh-password">Пароль</FieldLabel>
            <InputGroup>
              <InputGroupInput
                id="ssh-password"
                v-model="password"
                type="password"
                autocomplete="off"
                :placeholder="passwords[server.id] ? 'Сохранён — оставьте пустым, чтобы не менять' : 'Необязательно'"
                :aria-invalid="!!error || undefined"
              />
              <InputGroupAddon v-if="passwords[server.id]" align="inline-end">
                <InputGroupButton @click="forget(server.id)">
                  Забыть
                </InputGroupButton>
              </InputGroupAddon>
            </InputGroup>
            <FieldDescription>Пароль сервера или passphrase ключа — подставится на первый запрос. Хранится в ~/.vibecraft/credentials.json</FieldDescription>
            <FieldError :errors="[error]" />
          </Field>
          <Field>
            <FieldLabel for="ssh-args">Доп. флаги ssh</FieldLabel>
            <Input id="ssh-args" v-model="server.args" placeholder="-J bastion -L 8080:localhost:80" class="font-mono" />
          </Field>
        </FieldGroup>
        <DialogFooter>
          <Button type="submit" :disabled="busy || !server.host.trim()">
            <Spinner v-if="busy" data-icon="inline-start" />
            Сохранить
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
