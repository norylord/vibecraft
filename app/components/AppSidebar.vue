<script setup lang="ts">
import type { Worktree } from '@/composables/useProjects'
import { ChevronRightIcon, EllipsisIcon, EyeIcon, EyeOffIcon, FolderGit2Icon, GitBranchIcon, GitBranchPlusIcon, PlusIcon, RefreshCwIcon, SearchIcon, Settings2Icon, Trash2Icon, XIcon } from '@lucide/vue'
import { useEventListener } from '@vueuse/core'
import { toast } from 'vue-sonner'

const paletteOpen = useState('palette', () => false)
const { projects, hidden, collapsed, worktrees, activeWorktree, active, refresh, refreshAll, addProject, removeProject, toggleHidden, toggleCollapsed, createWorktree, removeWorktree } = useProjects()

const query = ref('')
const showHidden = ref(false)
// Поиск идёт и по скрытым: иначе спрятанный проект не найти.
// Проект с открытым worktree не прячем, иначе тот пропадёт из сайдбара
const shown = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (q) return projects.value.filter(p => baseName(p).toLowerCase().includes(q))
  return showHidden.value ? projects.value : projects.value.filter(p => !hidden.value.includes(p) || p === active.value?.repo)
})
// Последний скрытый вернули: кнопка исчезла, а без сброса следующее «Скрыть» ничего не спрячет
watch(() => hidden.value.length, (n) => {
  if (!n) showHidden.value = false
})
const view = useView()
const { shortcut, openSettings } = useSettings()
const { configured: integrations } = useIntegrations()

function select(path: string) {
  activeWorktree.value = path
  view.value = 'workspace'
}
const { statusIn } = useTerminals()

onMounted(refreshAll)
// Worktree могли создать или удалить из терминала — перечитываем при возврате в окно
useEventListener(window, 'focus', refreshAll)

const newWorktreeFor = ref<string>()
const branch = ref('')
const base = ref('')
const branchError = ref<string>()
const creating = ref(false)

function startNewWorktree(repo: string) {
  newWorktreeFor.value = repo
  branch.value = ''
  branchError.value = undefined
}

async function submitWorktree() {
  creating.value = true
  try {
    await createWorktree(newWorktreeFor.value!, branch.value.trim(), base.value)
    newWorktreeFor.value = undefined
  }
  catch (e) {
    branchError.value = String(e)
  }
  finally {
    creating.value = false
  }
}

const removal = ref<{ repo: string, wt: Worktree, force: boolean }>()

async function confirmRemoval() {
  const r = removal.value!
  try {
    await removeWorktree(r.repo, r.wt.path, r.force)
    removal.value = undefined
  }
  catch (e) {
    // git отказал из-за незакоммиченных изменений — спрашиваем второй раз, уже про них
    if (!r.force && String(e).includes('--force')) removal.value = { ...r, force: true }
    else {
      toast.error(String(e))
      removal.value = undefined
    }
  }
}
</script>

<template>
  <Sidebar variant="inset">
    <!-- pt-9: место под «светофор» macOS; пустая часть шапки таскает окно -->
    <SidebarHeader data-tauri-drag-region class="pt-9">
      <Button variant="outline" class="justify-start text-muted-foreground" @click="paletteOpen = true">
        <SearchIcon data-icon="inline-start" />
        Поиск и команды
        <Kbd class="ml-auto">{{ shortcut('palette') }}</Kbd>
      </Button>
    </SidebarHeader>
    <SidebarContent>
      <SidebarGroup>
        <SidebarGroupLabel>Проекты</SidebarGroupLabel>
        <SidebarGroupAction title="Добавить проект" @click="addProject">
          <PlusIcon />
          <span class="sr-only">Добавить проект</span>
        </SidebarGroupAction>
        <SidebarGroupContent>
          <SidebarInput v-if="projects.length" v-model="query" type="search" placeholder="Найти проект" aria-label="Найти проект" class="mb-1" />
          <SidebarMenu v-if="projects.length">
            <SidebarMenuItem v-for="repo in shown" :key="repo" :class="{ 'opacity-60': hidden.includes(repo) }">
              <!-- Основной worktree есть в списке ниже, поэтому клик по проекту только сворачивает -->
              <SidebarMenuButton
                :title="repo"
                :is-active="view === 'workspace' && collapsed.includes(repo) && active?.repo === repo"
                :aria-expanded="!collapsed.includes(repo)"
                @click="toggleCollapsed(repo)"
              >
                <FolderGit2Icon />
                <span class="truncate">{{ baseName(repo) }}</span>
                <ChevronRightIcon class="ml-auto transition-transform" :class="{ 'rotate-90': !collapsed.includes(repo) }" />
              </SidebarMenuButton>
              <DropdownMenu>
                <DropdownMenuTrigger as-child>
                  <SidebarMenuAction show-on-hover>
                    <EllipsisIcon />
                    <span class="sr-only">Действия с проектом</span>
                  </SidebarMenuAction>
                </DropdownMenuTrigger>
                <DropdownMenuContent side="right" align="start">
                  <DropdownMenuGroup>
                    <DropdownMenuItem @select="startNewWorktree(repo)">
                      <GitBranchPlusIcon />
                      Новый worktree
                    </DropdownMenuItem>
                    <DropdownMenuItem @select="refresh(repo)">
                      <RefreshCwIcon />
                      Обновить
                    </DropdownMenuItem>
                  </DropdownMenuGroup>
                  <DropdownMenuSeparator />
                  <DropdownMenuGroup>
                    <DropdownMenuItem @select="toggleHidden(repo)">
                      <component :is="hidden.includes(repo) ? EyeIcon : EyeOffIcon" />
                      {{ hidden.includes(repo) ? 'Показать' : 'Скрыть' }}
                    </DropdownMenuItem>
                    <DropdownMenuItem variant="destructive" @select="removeProject(repo)">
                      <XIcon />
                      Убрать из списка
                    </DropdownMenuItem>
                  </DropdownMenuGroup>
                </DropdownMenuContent>
              </DropdownMenu>
              <SidebarMenuSub v-if="!collapsed.includes(repo)">
                <SidebarMenuSubItem v-for="(wt, i) in worktrees[repo]" :key="wt.path">
                  <SidebarMenuSubButton as="button" class="w-full" :title="wt.path" :is-active="view === 'workspace' && wt.path === activeWorktree" @click="select(wt.path)">
                    <AgentStatusIcon :status="statusIn(wt.path)">
                      <GitBranchIcon />
                    </AgentStatusIcon>
                    <span>{{ wt.branch ?? 'detached HEAD' }}</span>
                  </SidebarMenuSubButton>
                  <!-- i > 0: основной worktree репозитория удалить нельзя -->
                  <Button
                    v-if="i > 0"
                    variant="ghost"
                    size="icon-xs"
                    class="absolute top-1 right-1 opacity-0 group-hover/menu-sub-item:opacity-100"
                    @click="removal = { repo, wt, force: false }"
                  >
                    <Trash2Icon />
                    <span class="sr-only">Удалить worktree</span>
                  </Button>
                </SidebarMenuSubItem>
              </SidebarMenuSub>
            </SidebarMenuItem>
            <SidebarMenuItem v-if="query.trim() && !shown.length">
              <p class="px-2 py-1.5 text-xs text-muted-foreground">Ничего не найдено</p>
            </SidebarMenuItem>
            <SidebarMenuItem v-if="hidden.length && !query.trim()">
              <SidebarMenuButton class="text-muted-foreground" @click="showHidden = !showHidden">
                <component :is="showHidden ? EyeOffIcon : EyeIcon" />
                <span>{{ showHidden ? 'Спрятать скрытые' : `Скрытые: ${hidden.length}` }}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
          <Empty v-else>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <FolderGit2Icon />
              </EmptyMedia>
              <EmptyTitle>Проектов пока нет</EmptyTitle>
              <EmptyDescription>Добавьте git-репозиторий, чтобы запускать агентов в worktree</EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button variant="outline" @click="addProject">
                <PlusIcon data-icon="inline-start" />
                Добавить проект
              </Button>
            </EmptyContent>
          </Empty>
        </SidebarGroupContent>
      </SidebarGroup>
      <!-- Ярлыки только у подключённых интеграций; настраиваются на странице «Настройки» -->
      <SidebarGroup v-if="integrations.length">
        <SidebarGroupLabel>Интеграции</SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            <SidebarMenuItem v-for="i in integrations" :key="i.id">
              <SidebarMenuButton :is-active="view === i.id" @click="view = i.id">
                <component :is="i.icon" />
                <span>{{ i.name }}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
    <SidebarFooter>
      <AgentUsage />
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton :is-active="view === 'settings'" @click="openSettings()">
            <Settings2Icon />
            <span>Настройки</span>
            <Kbd class="ml-auto">{{ shortcut('settings') }}</Kbd>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarFooter>
  </Sidebar>

  <Dialog :open="!!newWorktreeFor" @update:open="open => !open && (newWorktreeFor = undefined)">
    <DialogContent class="sm:max-w-sm">
      <form class="flex flex-col gap-4" @submit.prevent="submitWorktree">
        <DialogHeader>
          <DialogTitle>Новый worktree</DialogTitle>
          <DialogDescription>
            {{ baseName(newWorktreeFor ?? '') }}: если ветка уже есть — переиспользуем
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field :data-invalid="!!branchError || undefined">
            <FieldLabel for="branch">Ветка</FieldLabel>
            <Input id="branch" v-model="branch" placeholder="feat/new-thing" autofocus :aria-invalid="!!branchError || undefined" />
            <FieldError :errors="[branchError]" />
          </Field>
          <Field>
            <FieldLabel for="base">От ветки</FieldLabel>
            <BaseBranchSelect id="base" v-model="base" :repo="newWorktreeFor ?? ''" />
          </Field>
        </FieldGroup>
        <DialogFooter>
          <Button type="submit" :disabled="creating || !branch.trim() || !base">
            <Spinner v-if="creating" data-icon="inline-start" />
            Создать
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>

  <AlertDialog :open="!!removal" @update:open="open => !open && (removal = undefined)">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>Удалить worktree {{ removal?.wt.branch ?? 'detached HEAD' }}?</AlertDialogTitle>
        <AlertDialogDescription v-if="removal?.force">
          В нём есть незакоммиченные изменения — они будут потеряны безвозвратно.
        </AlertDialogDescription>
        <AlertDialogDescription v-else>
          Папка worktree будет удалена, ветка останется в репозитории. Открытые в нём терминалы закроются.
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>Отмена</AlertDialogCancel>
        <!-- Не AlertDialogAction: он закрывает диалог, а при отказе git нужен второй вопрос -->
        <Button variant="destructive" @click="confirmRemoval">
          {{ removal?.force ? 'Удалить с изменениями' : 'Удалить' }}
        </Button>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
