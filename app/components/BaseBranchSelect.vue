<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'

const props = defineProps<{ repo: string, id?: string }>()
const base = defineModel<string>({ required: true })
const { worktrees, lastBase } = useProjects()
const branches = ref<string[]>([])
const list = (repo: string) => invoke<string[]>('git_branches', { repo })

// По умолчанию: прошлый выбор, затем типичные основные ветки, затем ветка основного worktree
watch(() => props.repo, async (repo) => {
  branches.value = []
  base.value = ''
  if (!repo) return
  const local = await list(repo).catch((e): string[] => {
    toast.error(String(e))
    return []
  })
  if (repo !== props.repo) return
  branches.value = local
  const preferred = [lastBase.value[repo], 'main', 'master', 'dev', 'develop', worktrees.value[repo]?.[0]?.branch]
  base.value = preferred.find(b => b && local.includes(b)) ?? local[0] ?? ''
  // Пока выбирают ветку, подтягиваем remote: новые ветки появятся в списке, выбор не сбрасывается.
  // Ошибку fetch покажет создание worktree
  if (await fetchRemote(repo)) return
  const fresh = await list(repo).catch((): string[] => [])
  if (repo === props.repo && fresh.length) branches.value = fresh
}, { immediate: true })
</script>

<template>
  <Select v-model="base" :disabled="!branches.length">
    <SelectTrigger :id="id" class="w-full font-mono">
      <SelectValue placeholder="Загрузка веток…" />
    </SelectTrigger>
    <SelectContent>
      <SelectGroup>
        <SelectItem v-for="b in branches" :key="b" :value="b" class="font-mono">
          {{ b }}
        </SelectItem>
      </SelectGroup>
    </SelectContent>
  </Select>
</template>
