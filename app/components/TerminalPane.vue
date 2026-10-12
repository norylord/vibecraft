<script setup lang="ts">
import type { AgentStatus } from '@/composables/useTerminals'
import '@xterm/xterm/css/xterm.css'
import { Channel, invoke } from '@tauri-apps/api/core'
import { useResizeObserver } from '@vueuse/core'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { Terminal } from '@xterm/xterm'
import { toast } from 'vue-sonner'

// tabKey — ключ сессии в хосте терминалов; restored — переподключиться к ней, ничего не запуская
const props = defineProps<{ tabKey: string, cwd?: string, command?: string, tracked?: boolean, workingOnEnter?: boolean, restored?: boolean, active: boolean }>()
// idle — агента прервали (Esc), exited — агент вышел, остался shell
const emit = defineEmits<{ title: [title: string], exit: [code: number], status: [status: AgentStatus | 'idle' | 'exited'] }>()

const el = useTemplateRef<HTMLDivElement>('el')
const { settings, isDark } = useSettings()

// Фон = --background темы: терминал сливается с панелью. В светлой теме приглушаем
// «белые» ANSI-цвета — иначе они пропадают на белом фоне
const DARK = { background: '#09090b', foreground: '#fafafa', cursor: '#fafafa', selectionBackground: '#3f3f46' }
const LIGHT = { background: '#ffffff', foreground: '#09090b', cursor: '#09090b', selectionBackground: '#d4d4d8', white: '#a1a1aa', brightWhite: '#71717a' }

const term = new Terminal({
  fontFamily: '"Geist Mono Variable", ui-monospace, monospace',
  fontSize: settings.value.terminalFontSize,
  lineHeight: 1.2,
  cursorBlink: true,
  macOptionIsMeta: true,
  scrollback: 10_000,
  theme: isDark.value ? DARK : LIGHT,
})
const fit = new FitAddon()
term.loadAddon(fit)

// Тема и размер шрифта меняются в настройках на лету
watch(isDark, dark => (term.options.theme = dark ? DARK : LIGHT))
watch(() => settings.value.terminalFontSize, (size) => {
  term.options.fontSize = size
  fit.fit()
})

const key = props.tabKey
let connected = false
let opened = false
let disposed = false
let agentAlive = !!props.tracked
// Пока xterm разбирает хвост вывода из прошлого запуска, статусы копим и отдаём только последний:
// старые «закончил» не должны сыпаться уведомлениями
let replaying = false
let replayStatus: AgentStatus | 'exited' | undefined

// Статус от хуков агента: ESC ] 777 ; vibecraft ; <status> BEL (см. src-tauri/src/hooks.rs).
// Последовательность невидима — xterm её поглощает
term.parser.registerOscHandler(777, (data) => {
  if (!data.startsWith('vibecraft;')) return false
  const status = data.slice('vibecraft;'.length) as AgentStatus | 'exited'
  if (status === 'exited') agentAlive = false
  if (replaying) replayStatus = status
  else emit('status', status)
  return true
})

term.onData((data) => {
  if (!connected) return
  invoke('pty_write', { key, data })
  if (!agentAlive) return
  // Хук Stop при прерывании не срабатывает — иначе статус «работает» завис бы
  if (data === '\x1B') emit('status', 'idle')
  else if (props.workingOnEnter && data.includes('\r')) emit('status', 'working')
})
term.onResize(({ cols, rows }) => connected && invoke('pty_resize', { key, cols, rows }))
term.onTitleChange(title => emit('title', title))

// Первое сообщение — размер PTY и хвост вывода сессии (у новой пустой)
function restore(buf: ArrayBuffer) {
  const size = new DataView(buf)
  // Скрытая вкладка ещё не измерена: разбираем вывод в том размере, в каком его рисовал агент
  if (!opened) term.resize(size.getUint16(0), size.getUint16(2))
  if (buf.byteLength === 4) return
  replaying = true
  term.write(new Uint8Array(buf, 4), () => {
    replaying = false
    if (replayStatus) emit('status', replayStatus)
  })
}

// cols/rows = 0 — подключиться к сессии, не меняя размер PTY
async function connect(cols: number, rows: number) {
  const onData = new Channel<ArrayBuffer>()
  let first = true
  onData.onmessage = (buf) => {
    if (!first) return term.write(new Uint8Array(buf))
    first = false
    restore(buf)
  }
  const onExit = new Channel<number>()
  onExit.onmessage = code => emit('exit', code)

  try {
    // false — сессии уже нет: агент завершился, пока приложение было закрыто
    if (!await invoke<boolean>('pty_open', { key, cwd: props.cwd, cols, rows, spawn: !props.restored, onData, onExit })) return emit('exit', 0)
  }
  catch (e) {
    toast.error(`Терминал не открылся: ${e}`)
    return emit('exit', 1)
  }
  // Вкладку закрыли, пока shell стартовал
  if (disposed) return void invoke('pty_kill', { key })
  connected = true
  if (opened) invoke('pty_resize', { key, cols: term.cols, rows: term.rows })
  // Shell прочитает команду из буфера tty после загрузки rc; когда агент выйдет — останется shell
  if (props.command && !props.restored) invoke('pty_write', { key, data: `${props.command}\r` })
}

let started = false

async function start() {
  started = true
  // xterm меряет ширину символа при open — шрифт должен быть уже загружен
  await document.fonts.load('13px "Geist Mono Variable"')
  if (disposed) return
  term.open(el.value!)
  try {
    const webgl = new WebglAddon()
    webgl.onContextLoss(() => webgl.dispose())
    term.loadAddon(webgl)
  }
  catch {
    // нет WebGL — остаётся DOM-рендерер
  }
  fit.fit()
  opened = true
  if (!props.restored) await connect(term.cols, term.rows)
  if (props.active) term.focus()
}

// Восстановленную вкладку подключаем сразу, даже скрытую: статусы её агента нужны в сайдбаре
onMounted(() => props.restored && connect(0, 0))

useResizeObserver(el, () => {
  // Скрытый контейнер (display: none) имеет нулевой размер: xterm измерил бы символы как 0
  // и застрял на 80 колонках, а агент нарисовал бы узкий интерфейс. Поэтому открываем
  // терминал и запускаем shell только когда он реально виден, дальше — просто подгоняем размер
  if (!el.value?.offsetWidth) return
  if (started) fit.fit()
  else start()
})

watch(() => props.active, active => active && nextTick(() => term.focus()))

onBeforeUnmount(() => {
  disposed = true
  if (connected) invoke('pty_kill', { key })
  term.dispose()
})
</script>

<template>
  <div class="size-full py-1 pl-3">
    <div ref="el" class="size-full" />
  </div>
</template>
