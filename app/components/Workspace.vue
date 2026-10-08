<script setup lang="ts">
import type { AgentPreset } from '@/composables/useAgents'
import type { AgentStatus, TerminalTab } from '@/composables/useTerminals'
import type { ActionId } from '@/utils/hotkeys'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { toast } from 'vue-sonner'
import { BotIcon, CodeXmlIcon, FolderOpenIcon, PanelLeftIcon, PanelRightIcon, PlusIcon, Settings2Icon, SquareTerminalIcon, SunMoonIcon, XIcon, ZoomInIcon, ZoomOutIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import type { SplitterPanel } from 'reka-ui'
import { useEventListener } from '@vueuse/core'
import { useSidebar } from '@/components/ui/sidebar'
import { cn } from '@/lib/utils'

// hidden: открыта другая страница — workspace скрыт, но не выгружен, терминалы живут
defineProps<{ hidden: boolean }>()

const { toggleSidebar } = useSidebar()
const view = useView()
const { configured: integrations } = useIntegrations()
const paletteOpen = useState('palette', () => false)
// ResizablePanel пробрасывает методы SplitterPanel (collapse/expand)
const panel = useTemplateRef<InstanceType<typeof SplitterPanel>>('panel')

const { projects, activeWorktree, active, label } = useProjects()
const { terminals, activeTerminal, openTerminal: open, closeTerminal } = useTerminals()

// Вкладки — только активного worktree (без worktree — терминалы в домашней папке)
const visibleTerminals = computed(() => terminals.value.filter(t => t.cwd === activeWorktree.value))
watch(visibleTerminals, (list) => {
  if (!list.some(t => t.key === activeTerminal.value)) activeTerminal.value = list.at(-1)?.key
})

function openTerminal() {
  view.value = 'workspace'
  open(activeWorktree.value)
}

const { runnable: agents } = useAgents()
const { settings, isDark, binding, shortcut, openSettings, zoomBy } = useSettings()
// Агентов запускаем только в worktree — в домашней папке им нечего делать
function launch(agent: AgentPreset) {
  view.value = 'workspace'
  open(activeWorktree.value, agent)
}

// Без app — в Finder; редактор задаётся в настройках
async function openWorktree(app?: string) {
  if (!active.value) return
  try {
    await invoke('open_path', { path: active.value.wt.path, app })
  }
  catch (e) {
    toast.error(String(e))
  }
}

async function onStatus(tab: TerminalTab, status: AgentStatus | 'idle' | 'exited') {
  const next = status === 'idle' || status === 'exited' ? undefined : status
  // Хуки могут прислать один статус дважды (PermissionRequest + Notification) — уведомляем один раз
  if (tab.status === next) return
  tab.status = next
  if (next !== 'waiting' && next !== 'done') return
  // Пользователь и так смотрит на этого агента — не отвлекаем
  const focused = document.hasFocus()
  if (focused && tab.key === activeTerminal.value) return
  const title = `${tab.title}: ${next === 'waiting' ? 'ждёт ввода' : 'закончил'}`
  const body = tab.cwd ? label(tab.cwd) : ''
  if (focused) return void toast(title, { description: body })
  if (!settings.value.notifications) return
  if (await isPermissionGranted() || await requestPermission() === 'granted') sendNotification({ title, body })
}

// Старт задачи из YouTrack: правую панель убираем — агенту вся ширина (⌘J вернёт)
const wideTerminal = useState('wide-terminal', () => 0)
watch(wideTerminal, () => settings.value.collapsePanelOnStart && panel.value?.collapse())

function togglePanel() {
  if (panel.value?.isCollapsed) panel.value.expand()
  else panel.value?.collapse()
}

function run(action: () => void) {
  paletteOpen.value = false
  action()
}

// Все сочетания — в одном месте; назначаются в настройках «Горячие клавиши».
// По code, а не key: работают в любой раскладке
const hotkeys: Record<ActionId, () => void> = {
  palette: () => (paletteOpen.value = !paletteOpen.value),
  sidebar: toggleSidebar,
  panel: () => {
    view.value = 'workspace'
    togglePanel()
  },
  terminal: openTerminal,
  settings: () => openSettings(),
  zoomIn: () => zoomBy(0.1),
  zoomOut: () => zoomBy(-0.1),
  zoomReset: () => (settings.value.zoom = 1),
}
useEventListener('keydown', (e: KeyboardEvent) => {
  const combo = comboOf(e)
  const action = combo && ACTIONS.find(a => binding(a.id) === combo)
  if (!action) return
  e.preventDefault()
  hotkeys[action.id]()
})
</script>

<template>
  <!-- animate-in снова срабатывает при каждом возврате со страниц интеграций -->
  <SidebarInset v-show="!hidden" :class="cn('min-w-0 overflow-hidden', !hidden && 'animate-in fade-in duration-200')">
    <AppHeader>
      <template v-if="active">
        <span class="text-foreground">{{ baseName(active.repo) }}</span> / {{ active.wt.branch ?? 'detached HEAD' }}
      </template>
      <template v-else>Worktree не выбран</template>
      <template #actions>
        <template v-if="active">
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="icon-sm" @click="openWorktree(settings.editor)">
                <CodeXmlIcon />
                <span class="sr-only">Открыть в {{ settings.editor }}</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent>Открыть в {{ settings.editor }}</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button variant="ghost" size="icon-sm" @click="openWorktree()">
                <FolderOpenIcon />
                <span class="sr-only">Показать в Finder</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent>Показать в Finder</TooltipContent>
          </Tooltip>
        </template>
        <Tooltip>
          <TooltipTrigger as-child>
            <Button variant="ghost" size="icon-sm" @click="togglePanel">
              <PanelRightIcon />
              <span class="sr-only">Правая панель</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>Правая панель {{ shortcut('panel') }}</TooltipContent>
        </Tooltip>
      </template>
    </AppHeader>

    <ResizablePanelGroup direction="horizontal" auto-save-id="diogen-workspace" class="min-h-0 flex-1">
      <ResizablePanel :min-size="40">
        <Tabs v-model="activeTerminal" class="h-full gap-0">
          <div v-if="visibleTerminals.length" class="flex h-9 shrink-0 items-center gap-1 px-2">
            <TabsList variant="line">
              <div v-for="tab in visibleTerminals" :key="tab.key" class="group/tab flex items-center">
                <TabsTrigger :value="tab.key">
                  <AgentStatusIcon :status="tab.status">
                    <component :is="tab.command ? BotIcon : SquareTerminalIcon" />
                  </AgentStatusIcon>
                  {{ tab.title }}
                </TabsTrigger>
                <Button variant="ghost" size="icon-xs" class="opacity-0 group-hover/tab:opacity-100" @click="closeTerminal(tab.key)">
                  <XIcon />
                  <span class="sr-only">Закрыть терминал</span>
                </Button>
              </div>
            </TabsList>
            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <Button variant="ghost" size="icon-xs">
                  <PlusIcon />
                  <span class="sr-only">Новый терминал или агент</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuGroup>
                  <DropdownMenuItem @select="openTerminal">
                    <SquareTerminalIcon />
                    Терминал
                    <DropdownMenuShortcut>{{ shortcut('terminal') }}</DropdownMenuShortcut>
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <template v-if="active">
                  <DropdownMenuSeparator />
                  <DropdownMenuGroup>
                    <DropdownMenuItem v-for="agent in agents" :key="agent.name" @select="launch(agent)">
                      <BotIcon />
                      {{ agent.name }}
                    </DropdownMenuItem>
                  </DropdownMenuGroup>
                  <DropdownMenuSeparator />
                  <DropdownMenuGroup>
                    <DropdownMenuItem @select="openSettings('agents')">
                      <Settings2Icon />
                      Настроить агентов…
                    </DropdownMenuItem>
                  </DropdownMenuGroup>
                </template>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <Separator v-if="visibleTerminals.length" />
          <WelcomePanel v-else-if="!projects.length" class="flex-1" @agents="openSettings('agents')" />
          <Empty v-else class="flex-1">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <SquareTerminalIcon />
              </EmptyMedia>
              <EmptyTitle>{{ active ? 'Нет запущенных агентов' : 'Worktree не выбран' }}</EmptyTitle>
              <EmptyDescription>
                {{ active ? 'Запустите агента или откройте терминал в этом worktree' : 'Выберите worktree в сайдбаре или откройте терминал в домашней папке' }}
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <div v-if="active" class="flex flex-wrap justify-center gap-2">
                <Button v-for="agent in agents" :key="agent.name" @click="launch(agent)">
                  <BotIcon data-icon="inline-start" />
                  {{ agent.name }}
                </Button>
              </div>
              <Button variant="outline" @click="openTerminal">
                <SquareTerminalIcon data-icon="inline-start" />
                Новый терминал
                <Kbd>{{ shortcut('terminal') }}</Kbd>
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
              :command="tab.command"
              :active="tab.key === activeTerminal"
              @title="tab.title = $event"
              :tracked="tab.tracked"
              :working-on-enter="tab.workingOnEnter"
              @status="onStatus(tab, $event)"
              @exit="closeTerminal(tab.key)"
            />
          </TabsContent>
        </Tabs>
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel ref="panel" collapsible :collapsed-size="0" :min-size="20" :default-size="32">
        <Tabs default-value="diff" class="h-full p-2">
          <TabsList variant="line">
            <TabsTrigger value="diff">
              Изменения
            </TabsTrigger>
            <TabsTrigger value="task">
              Задача
            </TabsTrigger>
            <TabsTrigger value="mr">
              MR
            </TabsTrigger>
          </TabsList>
          <TabsContent value="diff" class="min-h-0">
            <ChangesPanel />
          </TabsContent>
          <TabsContent value="task" class="min-h-0">
            <TaskPanel />
          </TabsContent>
          <TabsContent value="mr" class="min-h-0">
            <MergeRequestPanel />
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
          <CommandShortcut>{{ shortcut('terminal') }}</CommandShortcut>
        </CommandItem>
      </CommandGroup>
      <CommandGroup heading="Агенты">
        <template v-if="active">
          <CommandItem v-for="agent in agents" :key="agent.name" :value="`agent-${agent.name}`" @select="run(() => launch(agent))">
            <BotIcon />
            Запустить {{ agent.name }}
          </CommandItem>
        </template>
        <CommandItem value="agents-settings" @select="run(() => openSettings('agents'))">
          <Settings2Icon />
          Настроить агентов
        </CommandItem>
      </CommandGroup>
      <CommandGroup v-if="active" heading="Worktree">
        <CommandItem value="open-webstorm" @select="run(() => openWorktree(settings.editor))">
          <CodeXmlIcon />
          Открыть в {{ settings.editor }}
        </CommandItem>
        <CommandItem value="open-finder" @select="run(() => openWorktree())">
          <FolderOpenIcon />
          Показать в Finder
        </CommandItem>
      </CommandGroup>
      <CommandGroup heading="Интеграции">
        <CommandItem v-for="i in integrations" :key="i.id" :value="`integration-${i.id}`" @select="run(() => view = i.id)">
          <component :is="i.icon" />
          Открыть {{ i.name }}
        </CommandItem>
        <CommandItem value="settings" @select="run(() => openSettings())">
          <Settings2Icon />
          Настройки
          <CommandShortcut>{{ shortcut('settings') }}</CommandShortcut>
        </CommandItem>
      </CommandGroup>
      <CommandGroup heading="Вид">
        <CommandItem value="sidebar" @select="run(toggleSidebar)">
          <PanelLeftIcon />
          Сайдбар
          <CommandShortcut>{{ shortcut('sidebar') }}</CommandShortcut>
        </CommandItem>
        <CommandItem value="panel" @select="run(togglePanel)">
          <PanelRightIcon />
          Правая панель
          <CommandShortcut>{{ shortcut('panel') }}</CommandShortcut>
        </CommandItem>
        <CommandItem value="theme" @select="run(() => settings.theme = isDark ? 'light' : 'dark')">
          <SunMoonIcon />
          {{ isDark ? 'Светлая тема' : 'Тёмная тема' }}
        </CommandItem>
        <CommandItem value="zoom-in" @select="run(() => zoomBy(0.1))">
          <ZoomInIcon />
          Увеличить масштаб
          <CommandShortcut>{{ shortcut('zoomIn') }}</CommandShortcut>
        </CommandItem>
        <CommandItem value="zoom-out" @select="run(() => zoomBy(-0.1))">
          <ZoomOutIcon />
          Уменьшить масштаб
          <CommandShortcut>{{ shortcut('zoomOut') }}</CommandShortcut>
        </CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>

</template>
