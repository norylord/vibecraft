<script setup lang="ts">
import { GitCompareIcon, GitMergeIcon, PanelLeftIcon, PanelRightIcon, PlusIcon, SquareTerminalIcon, TicketIcon, XIcon } from '@lucide/vue'
import type { SplitterPanel } from 'reka-ui'
import { useEventListener } from '@vueuse/core'
import { useSidebar } from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'

const { state, toggleSidebar } = useSidebar()
const paletteOpen = useState('palette', () => false)
// ResizablePanel пробрасывает методы SplitterPanel (collapse/expand)
const panel = useTemplateRef<InstanceType<typeof SplitterPanel>>('panel')

const panelTabs = [
  { value: 'diff', label: 'Изменения', icon: GitCompareIcon, title: 'Нет изменений', description: 'Здесь появится diff активного worktree' },
  { value: 'task', label: 'Задача', icon: TicketIcon, title: 'Задача не привязана', description: 'Запустите агента из задачи YouTrack' },
  { value: 'mr', label: 'MR', icon: GitMergeIcon, title: 'MR не создан', description: 'Создайте merge request в GitLab из ветки worktree' },
]

const { activeWorktree, active } = useProjects()
const { terminals, activeTerminal, openTerminal: open, closeTerminal } = useTerminals()

// Вкладки — только активного worktree (без worktree — терминалы в домашней папке)
const visibleTerminals = computed(() => terminals.value.filter(t => t.cwd === activeWorktree.value))
watch(visibleTerminals, (list) => {
  if (!list.some(t => t.key === activeTerminal.value)) activeTerminal.value = list.at(-1)?.key
})

const openTerminal = () => open(activeWorktree.value)

function togglePanel() {
  if (panel.value?.isCollapsed) panel.value.expand()
  else panel.value?.collapse()
}

function run(action: () => void) {
  paletteOpen.value = false
  action()
}

// code, а не key — хоткеи работают в любой раскладке
useEventListener('keydown', (e: KeyboardEvent) => {
  if (!e.metaKey) return
  if (e.code === 'KeyK') {
    e.preventDefault()
    paletteOpen.value = !paletteOpen.value
  }
  if (e.code === 'KeyJ') {
    e.preventDefault()
    togglePanel()
  }
  if (e.code === 'KeyT') {
    e.preventDefault()
    openTerminal()
  }
})
</script>

<template>
  <SidebarInset class="min-w-0 overflow-hidden">
    <!-- pl-20 при скрытом сайдбаре: не залезать под «светофор» macOS -->
    <header
      data-tauri-drag-region
      :class="cn('flex h-10 shrink-0 items-center gap-2 px-2 transition-[padding]', state === 'collapsed' && 'pl-20')"
    >
      <SidebarTrigger />
      <span data-tauri-drag-region class="truncate text-xs text-muted-foreground">
        <template v-if="active">
          <span class="text-foreground">{{ baseName(active.repo) }}</span> / {{ active.wt.branch ?? 'detached HEAD' }}
        </template>
        <template v-else>Worktree не выбран</template>
      </span>
      <Button variant="ghost" size="icon-sm" class="ml-auto" @click="togglePanel">
        <PanelRightIcon />
        <span class="sr-only">Правая панель</span>
      </Button>
    </header>
    <Separator />

    <ResizablePanelGroup direction="horizontal" auto-save-id="diogen-workspace" class="min-h-0 flex-1">
      <ResizablePanel :min-size="40">
        <Tabs v-model="activeTerminal" class="h-full gap-0">
          <div v-if="visibleTerminals.length" class="flex h-9 shrink-0 items-center gap-1 px-2">
            <TabsList variant="line">
              <div v-for="tab in visibleTerminals" :key="tab.key" class="group/tab flex items-center">
                <TabsTrigger :value="tab.key">
                  <SquareTerminalIcon />
                  {{ tab.title }}
                </TabsTrigger>
                <Button variant="ghost" size="icon-xs" class="opacity-0 group-hover/tab:opacity-100" @click="closeTerminal(tab.key)">
                  <XIcon />
                  <span class="sr-only">Закрыть терминал</span>
                </Button>
              </div>
            </TabsList>
            <Button variant="ghost" size="icon-xs" @click="openTerminal">
              <PlusIcon />
              <span class="sr-only">Новый терминал</span>
            </Button>
          </div>
          <Separator v-if="visibleTerminals.length" />
          <Empty v-else class="flex-1">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <SquareTerminalIcon />
              </EmptyMedia>
              <EmptyTitle>{{ active ? 'Нет запущенных агентов' : 'Worktree не выбран' }}</EmptyTitle>
              <EmptyDescription>
                {{ active ? 'Откройте терминал и запустите Claude Code, Codex или Gemini' : 'Выберите worktree в сайдбаре или откройте терминал в домашней папке' }}
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button variant="outline" @click="openTerminal">
                <SquareTerminalIcon data-icon="inline-start" />
                Новый терминал
                <Kbd>⌘T</Kbd>
              </Button>
            </EmptyContent>
          </Empty>
          <!-- Все терминалы всех worktree смонтированы (force-mount): скрытые живут, shell не перезапускается -->
          <TabsContent
            v-for="tab in terminals"
            :key="tab.key"
            :value="tab.key"
            force-mount
            class="min-h-0 data-[state=inactive]:hidden"
          >
            <TerminalPane
              :cwd="tab.cwd"
              :active="tab.key === activeTerminal"
              @title="tab.title = $event"
              @exit="closeTerminal(tab.key)"
            />
          </TabsContent>
        </Tabs>
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel ref="panel" collapsible :collapsed-size="0" :min-size="20" :default-size="32">
        <Tabs default-value="diff" class="h-full p-2">
          <TabsList variant="line">
            <TabsTrigger v-for="tab in panelTabs" :key="tab.value" :value="tab.value">
              {{ tab.label }}
            </TabsTrigger>
          </TabsList>
          <TabsContent v-for="tab in panelTabs" :key="tab.value" :value="tab.value">
            <Empty class="h-full">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <component :is="tab.icon" />
                </EmptyMedia>
                <EmptyTitle>{{ tab.title }}</EmptyTitle>
                <EmptyDescription>{{ tab.description }}</EmptyDescription>
              </EmptyHeader>
            </Empty>
          </TabsContent>
        </Tabs>
      </ResizablePanel>
    </ResizablePanelGroup>
  </SidebarInset>

  <CommandDialog v-model:open="paletteOpen">
    <CommandInput placeholder="Введите команду…" />
    <CommandList>
      <CommandEmpty>Ничего не найдено</CommandEmpty>
      <CommandGroup heading="Терминал">
        <CommandItem value="terminal" @select="run(openTerminal)">
          <SquareTerminalIcon />
          Новый терминал
          <CommandShortcut>⌘T</CommandShortcut>
        </CommandItem>
      </CommandGroup>
      <CommandGroup heading="Вид">
        <CommandItem value="sidebar" @select="run(toggleSidebar)">
          <PanelLeftIcon />
          Сайдбар
          <CommandShortcut>⌘B</CommandShortcut>
        </CommandItem>
        <CommandItem value="panel" @select="run(togglePanel)">
          <PanelRightIcon />
          Правая панель
          <CommandShortcut>⌘J</CommandShortcut>
        </CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>
</template>
