<script setup lang="ts">
import '@xterm/xterm/css/xterm.css'
import { Channel, invoke } from '@tauri-apps/api/core'
import { useResizeObserver } from '@vueuse/core'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { Terminal } from '@xterm/xterm'

const props = defineProps<{ cwd?: string, command?: string, active: boolean }>()
const emit = defineEmits<{ title: [title: string], exit: [code: number] }>()

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

onMounted(async () => {
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

  term.onData(data => invoke('pty_write', { id, data }))
  term.onResize(({ cols, rows }) => invoke('pty_resize', { id, cols, rows }))
  term.onTitleChange(title => emit('title', title))
  // Shell прочитает команду из буфера tty после загрузки rc; когда агент выйдет — останется shell
  if (props.command) invoke('pty_write', { id, data: `${props.command}\r` })
  if (props.active) term.focus()
})

useResizeObserver(el, () => {
  // Скрытая вкладка имеет нулевой размер — fit посчитал бы 0 колонок
  if (el.value?.offsetWidth) fit.fit()
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
