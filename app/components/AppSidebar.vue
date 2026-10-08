<script setup lang="ts">
import type { Worktree } from '@/composables/useProjects'
import { EllipsisIcon, FolderGit2Icon, GitBranchIcon, GitBranchPlusIcon, PlusIcon, RefreshCwIcon, SearchIcon, Settings2Icon, Trash2Icon, XIcon } from '@lucide/vue'
import { useEventListener } from '@vueuse/core'
import { toast } from 'vue-sonner'

const paletteOpen = useState('palette', () => false)
const { projects, worktrees, activeWorktree, refresh, refreshAll, addProject, removeProject, createWorktree, removeWorktree } = useProjects()
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
    await createWorktree(newWorktreeFor.value!, branch.value.trim())
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
          <SidebarMenu v-if="projects.length">
            <SidebarMenuItem v-for="repo in projects" :key="repo">
              <SidebarMenuButton :title="repo" @click="select(repo)">
                <FolderGit2Icon />
                <span>{{ baseName(repo) }}</span>
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
                    <DropdownMenuItem variant="destructive" @select="removeProject(repo)">
                      <XIcon />
                      Убрать из списка
                    </DropdownMenuItem>
                  </DropdownMenuGroup>
                </DropdownMenuContent>
              </DropdownMenu>
              <SidebarMenuSub>
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
            {{ baseName(newWorktreeFor ?? '') }}: ветка создастся от текущего HEAD, если её ещё нет
          </DialogDescription>
        </DialogHeader>
        <FieldGroup>
          <Field :data-invalid="!!branchError || undefined">
            <FieldLabel for="branch">Ветка</FieldLabel>
            <Input id="branch" v-model="branch" placeholder="feat/new-thing" autofocus :aria-invalid="!!branchError || undefined" />
            <FieldError :errors="[branchError]" />
          </Field>
        </FieldGroup>
        <DialogFooter>
          <Button type="submit" :disabled="creating || !branch.trim()">
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
