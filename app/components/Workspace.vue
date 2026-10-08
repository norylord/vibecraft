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

interface TerminalTab { key: string, title: string, cwd?: string }
const terminals = ref<TerminalTab[]>([])
const activeTerminal = ref<string>()

function openTerminal(cwd?: string) {
  const tab = { key: crypto.randomUUID(), title: 'Терминал', cwd }
  terminals.value.push(tab)
  activeTerminal.value = tab.key
}

function closeTerminal(key: string) {
  const i = terminals.value.findIndex(t => t.key === key)
  // Закрытие по × убивает shell, и следом приходит его exit — второй раз ничего не делаем
  if (i === -1) return
  terminals.value.splice(i, 1)
  if (activeTerminal.value === key) activeTerminal.value = terminals.value[Math.min(i, terminals.value.length - 1)]?.key
}

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
      <span data-tauri-drag-region class="truncate text-xs text-muted-foreground">Worktree не выбран</span>
      <Button variant="ghost" size="icon-sm" class="ml-auto" @click="togglePanel">
        <PanelRightIcon />
        <span class="sr-only">Правая панель</span>
      </Button>
    </header>
    <Separator />

    <ResizablePanelGroup direction="horizontal" auto-save-id="diogen-workspace" class="min-h-0 flex-1">
      <ResizablePanel :min-size="40">
        <Tabs v-if="terminals.length" v-model="activeTerminal" class="h-full gap-0">
          <div class="flex h-9 shrink-0 items-center gap-1 px-2">
            <TabsList variant="line">
              <div v-for="tab in terminals" :key="tab.key" class="group/tab flex items-center">
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
            <Button variant="ghost" size="icon-xs" @click="openTerminal()">
              <PlusIcon />
              <span class="sr-only">Новый терминал</span>
            </Button>
          </div>
          <Separator />
          <!-- force-mount: неактивные терминалы живут скрытыми, shell не перезапускается -->
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
        <Empty v-else class="h-full">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <SquareTerminalIcon />
            </EmptyMedia>
            <EmptyTitle>Нет запущенных агентов</EmptyTitle>
            <EmptyDescription>Выберите worktree и запустите Claude Code, Codex или Gemini</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button variant="outline" @click="openTerminal()">
              <SquareTerminalIcon data-icon="inline-start" />
              Новый терминал
              <Kbd>⌘T</Kbd>
            </Button>
          </EmptyContent>
        </Empty>
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
