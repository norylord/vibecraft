<script setup lang="ts">
import type { AgentStatus } from '@/composables/useTerminals'
import '@xterm/xterm/css/xterm.css'
import { Channel, invoke } from '@tauri-apps/api/core'
import { useResizeObserver } from '@vueuse/core'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { Terminal } from '@xterm/xterm'

const props = defineProps<{ cwd?: string, command?: string, tracked?: boolean, workingOnEnter?: boolean, active: boolean }>()
// idle — агента прервали (Esc), exited — агент вышел, остался shell
const emit = defineEmits<{ title: [title: string], exit: [code: number], status: [status: AgentStatus | 'idle' | 'exited'] }>()

const el = useTemplateRef<HTMLDivElement>('el')
// Фон = --background (zinc-950): терминал сливается с панелью
const term = new Terminal({
  fontFamily: '"Geist Mono Variable", ui-monospace, monospace',
  fontSize: 13,
  lineHeight: 1.2,
  cursorBlink: true,
  macOptionIsMeta: true,
  scrollback: 10_000,
  theme: { background: '#09090b', foreground: '#fafafa', cursor: '#fafafa', selectionBackground: '#3f3f46' },
})
const fit = new FitAddon()
term.loadAddon(fit)

let id: number | undefined
let disposed = false
let agentAlive = !!props.tracked

// Статус от хуков агента: ESC ] 777 ; diogen ; <status> BEL (см. src-tauri/src/hooks.rs).
// Последовательность невидима — xterm её поглощает
term.parser.registerOscHandler(777, (data) => {
  if (!data.startsWith('diogen;')) return false
  const status = data.slice('diogen;'.length) as AgentStatus | 'exited'
  if (status === 'exited') agentAlive = false
  emit('status', status)
  return true
})

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

  const onData = new Channel<ArrayBuffer>()
  onData.onmessage = buf => term.write(new Uint8Array(buf))
  const onExit = new Channel<number>()
  onExit.onmessage = code => emit('exit', code)

  const ptyId = await invoke<number>('pty_spawn', { cwd: props.cwd, cols: term.cols, rows: term.rows, onData, onExit })
  // Вкладку закрыли, пока shell стартовал
  if (disposed) return void invoke('pty_kill', { id: ptyId })
  id = ptyId

  term.onData((data) => {
    invoke('pty_write', { id, data })
    if (!agentAlive) return
    // Хук Stop при прерывании не срабатывает — иначе статус «работает» завис бы
    if (data === '\x1B') emit('status', 'idle')
    else if (props.workingOnEnter && data.includes('\r')) emit('status', 'working')
  })
  term.onResize(({ cols, rows }) => invoke('pty_resize', { id, cols, rows }))
  term.onTitleChange(title => emit('title', title))
  // Shell прочитает команду из буфера tty после загрузки rc; когда агент выйдет — останется shell
  if (props.command) invoke('pty_write', { id, data: `${props.command}\r` })
  if (props.active) term.focus()
}

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
  if (id !== undefined) invoke('pty_kill', { id })
  term.dispose()
})
</script>

<template>
  <div class="size-full py-1 pl-3">
    <div ref="el" class="size-full" />
  </div>
</template>
