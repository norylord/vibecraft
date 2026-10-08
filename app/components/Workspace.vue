<script setup lang="ts">
import { GitCompareIcon, GitMergeIcon, PanelLeftIcon, PanelRightIcon, SquareTerminalIcon, TicketIcon } from '@lucide/vue'
import type { SplitterPanel } from 'reka-ui'
import { useEventListener } from '@vueuse/core'
import { useSidebar } from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'

const { state, toggleSidebar } = useSidebar()
const paletteOpen = useState('palette', () => false)
// ResizablePanel пробрасывает методы SplitterPanel (collapse/expand)
const panel = useTemplateRef<InstanceType<typeof SplitterPanel>>('panel')

const tabs = [
  { value: 'diff', label: 'Изменения', icon: GitCompareIcon, title: 'Нет изменений', description: 'Здесь появится diff активного worktree' },
  { value: 'task', label: 'Задача', icon: TicketIcon, title: 'Задача не привязана', description: 'Запустите агента из задачи YouTrack' },
  { value: 'mr', label: 'MR', icon: GitMergeIcon, title: 'MR не создан', description: 'Создайте merge request в GitLab из ветки worktree' },
]

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
        <Empty class="h-full">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <SquareTerminalIcon />
            </EmptyMedia>
            <EmptyTitle>Нет запущенных агентов</EmptyTitle>
            <EmptyDescription>Выберите worktree и запустите Claude Code, Codex или Gemini</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <KbdGroup>
              <Kbd>⌘</Kbd>
              <Kbd>K</Kbd>
            </KbdGroup>
          </EmptyContent>
        </Empty>
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel ref="panel" collapsible :collapsed-size="0" :min-size="20" :default-size="32">
        <Tabs default-value="diff" class="h-full p-2">
          <TabsList variant="line">
            <TabsTrigger v-for="tab in tabs" :key="tab.value" :value="tab.value">
              {{ tab.label }}
            </TabsTrigger>
          </TabsList>
          <TabsContent v-for="tab in tabs" :key="tab.value" :value="tab.value">
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
