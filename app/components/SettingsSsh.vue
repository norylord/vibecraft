<script setup lang="ts">
import type { SshServer } from '@/utils/ssh'
import { FileInputIcon, PencilIcon, PlugIcon, PlusIcon, Trash2Icon } from '@lucide/vue'
import { ask } from '@tauri-apps/plugin-dialog'
import { toast } from 'vue-sonner'

const { servers, passwords, remove, connect, importConfig } = useSsh()
const editing = ref<SshServer>()

const add = () => (editing.value = { id: crypto.randomUUID(), name: '', host: '' })
const address = (s: SshServer) => `${s.user ? `${s.user}@` : ''}${s.host}${s.port ? `:${s.port}` : ''}`

async function onRemove(s: SshServer) {
  if (await ask(`Сервер «${s.name || s.host}» и его сохранённый пароль будут удалены`, { title: 'Удалить сервер?', kind: 'warning', okLabel: 'Удалить', cancelLabel: 'Отмена' })) {
    await remove(s).catch(e => toast.error(String(e)))
  }
}

async function onImport() {
  try {
    const added = await importConfig()
    if (added) toast.success(`Добавлено хостов: ${added}`)
    else toast.info('Новых хостов в ~/.ssh/config нет')
  }
  catch (e) {
    toast.error(String(e))
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="text-xs text-muted-foreground">
      Сервер открывается вкладкой терминала в текущем worktree через системный <code class="font-mono">ssh</code>:
      ssh-agent, <code class="font-mono">~/.ssh/config</code> и known_hosts работают как в обычном терминале.
    </p>
    <div v-if="servers.length" class="flex flex-col divide-y rounded-lg border">
      <div v-for="s in servers" :key="s.id" class="flex items-center gap-2 py-2 pr-2 pl-3">
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm font-medium">
            {{ s.name || s.host }}
          </div>
          <div class="truncate font-mono text-xs text-muted-foreground">
            {{ address(s) }}
          </div>
        </div>
        <Badge v-if="s.key" variant="outline">
          ключ
        </Badge>
        <Badge v-if="passwords[s.id]" variant="outline">
          пароль
        </Badge>
        <Button variant="ghost" size="icon" @click="connect(s)">
          <PlugIcon />
          <span class="sr-only">Подключиться</span>
        </Button>
        <Button variant="ghost" size="icon" @click="editing = { ...s }">
          <PencilIcon />
          <span class="sr-only">Изменить</span>
        </Button>
        <Button variant="ghost" size="icon" @click="onRemove(s)">
          <Trash2Icon />
          <span class="sr-only">Удалить</span>
        </Button>
      </div>
    </div>
    <div class="flex gap-2">
      <Button variant="outline" @click="add">
        <PlusIcon data-icon="inline-start" />
        Добавить сервер
      </Button>
      <Button variant="ghost" @click="onImport">
        <FileInputIcon data-icon="inline-start" />
        Импорт из ~/.ssh/config
      </Button>
    </div>
    <SshServerDialog v-model:server="editing" />
  </div>
</template>
