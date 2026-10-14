<script setup lang="ts">
import { LanguageDescription, syntaxHighlighting } from '@codemirror/language'
import { languages } from '@codemirror/language-data'
import { Chunk } from '@codemirror/merge'
import { Compartment, EditorState, Prec, RangeSet, StateEffect, StateField, Text, Transaction } from '@codemirror/state'
import { oneDarkHighlightStyle } from '@codemirror/theme-one-dark'
import { gutter, GutterMarker, keymap } from '@codemirror/view'
import { FileWarningIcon, TriangleAlertIcon } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, useIntervalFn } from '@vueuse/core'
import { basicSetup, EditorView } from 'codemirror'
import { toast } from 'vue-sonner'

// path — относительно root (корня worktree)
const props = defineProps<{ root: string, path: string, active: boolean }>()
const emit = defineEmits<{ dirty: [dirty: boolean] }>()

const el = useTemplateRef<HTMLDivElement>('el')
const { settings, isDark } = useSettings()
const error = ref<string>()
const dirty = ref(false)
// Файл изменили на диске (обычно агент), пока здесь есть несохранённые правки
const conflict = ref(false)
watch(dirty, d => emit('dirty', d))

const args = { root: props.root, path: props.path }
let view: EditorView | undefined
let disposed = false
// Пока идёт запись или чтение, опрос не трогает файл: иначе принял бы своё сохранение за правку агента
let busy = false
// Текст, совпадающий с диском, и время его изменения
let saved = Text.empty
let mtime: number | undefined
// id версии файла в HEAD: сменился — перечитываем, с чем сравнивать
let head: string | null | undefined

// Отметки строк, изменённых относительно HEAD (как в IDE): добавлено, изменено, удалено
class ChangeMark extends GutterMarker {
  constructor(readonly kind: 'add' | 'mod' | 'del') {
    super()
  }

  override eq(other: ChangeMark) {
    return other.kind === this.kind
  }

  override toDOM() {
    const mark = document.createElement('div')
    mark.className = `cm-change cm-change-${this.kind}`
    return mark
  }
}
const MARKS = { add: new ChangeMark('add'), mod: new ChangeMark('mod'), del: new ChangeMark('del') }

const setOriginal = StateEffect.define<Text | null>()
const original = StateField.define<Text | null>({
  create: () => null,
  update(value, tr) {
    for (const e of tr.effects) {
      if (e.is(setOriginal)) value = e.value
    }
    return value
  },
})

// Чанки идут по целым строкам; to* — сразу за последней строкой чанка и может выходить за конец документа
function changeMarks(a: Text, b: Text) {
  const ranges = []
  for (const c of Chunk.build(a, b)) {
    if (c.fromB === c.toB) {
      ranges.push(MARKS.del.range(b.lineAt(Math.min(c.fromB, b.length)).from))
      continue
    }
    const mark = c.fromA === c.toA ? MARKS.add : MARKS.mod
    const last = b.lineAt(Math.min(c.toB - 1, b.length)).number
    for (let n = b.lineAt(c.fromB).number; n <= last; n++) ranges.push(mark.range(b.line(n).from))
  }
  return RangeSet.of(ranges, true)
}

const changes = StateField.define<RangeSet<GutterMarker>>({
  create: () => RangeSet.empty,
  update(marks, tr) {
    const base = tr.state.field(original)
    if (!tr.docChanged && base === tr.startState.field(original)) return marks
    return base ? changeMarks(base, tr.state.doc) : RangeSet.empty
  },
})

// Фон и шрифт — как у терминала: редактор сливается с панелью
const theme = EditorView.theme({
  '&': { height: '100%', backgroundColor: 'var(--background)', color: 'var(--foreground)' },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': { fontFamily: '"Geist Mono Variable", ui-monospace, monospace', lineHeight: '1.5' },
  '.cm-content': { caretColor: 'var(--foreground)' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--foreground)' },
  '.cm-gutters': { backgroundColor: 'var(--background)', color: 'var(--muted-foreground)', border: 'none' },
  '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'color-mix(in oklab, var(--muted) 60%, transparent)' },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
    backgroundColor: 'color-mix(in oklab, var(--ring) 35%, transparent)',
  },
  '.cm-changes': { width: '3px', marginRight: '6px' },
  '.cm-change': { width: '3px', height: '100%' },
  '.cm-change-add': { backgroundColor: 'var(--success)' },
  '.cm-change-mod': { backgroundColor: 'var(--warning)' },
  '.cm-change-del': { width: '0', height: '0', borderTop: '4px solid transparent', borderBottom: '4px solid transparent', borderLeft: '5px solid var(--destructive)', transform: 'translateY(-4px)' },
})

const language = new Compartment()
const appearance = new Compartment()
// Светлой теме хватает подсветки CodeMirror по умолчанию, в тёмной её цвета нечитаемы
const look = (dark: boolean) => [EditorView.darkTheme.of(dark), dark ? syntaxHighlighting(oneDarkHighlightStyle) : []]
watch(isDark, dark => view?.dispatch({ effects: appearance.reconfigure(look(dark)) }))
watch(() => settings.value.terminalFontSize, () => view?.requestMeasure())

type File = { text: string, mtime: number }

async function save(force = false) {
  if (!view || busy) return
  busy = true
  const doc = view.state.doc
  try {
    // sliceDoc, а не doc.toString(): сохраняет переводы строк файла (\r\n)
    mtime = await invoke<number>('fs_write', { ...args, text: view.state.sliceDoc(), expected: force ? null : mtime })
    saved = doc
    dirty.value = !view.state.doc.eq(saved)
    conflict.value = false
  }
  catch (e) {
    if (e === 'conflict') conflict.value = true
    else toast.error(`${props.path} не сохранён: ${e}`)
  }
  finally {
    busy = false
  }
}

// Текст с диска, поверх несохранённых правок
async function reload() {
  if (!view || busy) return
  busy = true
  try {
    const file = await invoke<File>('fs_read', args)
    if (!view) return
    const { doc, lineBreak } = view.state
    const next = view.state.toText(file.text)
    // Только изменённые куски: курсор и прокрутка остаются на месте. Вне истории: ⌘Z не откатит правки агента
    const changes = Chunk.build(doc, next).map(c => ({
      from: c.fromA,
      to: Math.min(c.toA, doc.length),
      insert: next.sliceString(c.fromB, Math.min(c.toB, next.length), lineBreak),
    }))
    saved = next
    mtime = file.mtime
    view.dispatch({ changes, annotations: Transaction.addToHistory.of(false) })
    dirty.value = false
    conflict.value = false
  }
  catch (e) {
    toast.error(`${props.path}: ${e}`)
  }
  finally {
    busy = false
  }
}

// Опрос: агент поменял файл на диске или кто-то закоммитил (сменилась версия в HEAD)
async function check() {
  if (!view || busy) return
  const stat = await invoke<{ mtime: number | null, head: string | null }>('fs_stat', args)
  if (!view) return
  if (stat.head !== head) {
    head = stat.head
    const text = head ? await invoke<string | null>('fs_head', args) : null
    view?.dispatch({ effects: setOriginal.of(text === null ? null : view.state.toText(text)) })
  }
  if (stat.mtime === null || stat.mtime === mtime) return
  if (dirty.value) conflict.value = true
  else await reload()
}

onMounted(async () => {
  let file: File
  try {
    file = await invoke<File>('fs_read', args)
  }
  catch (e) {
    error.value = String(e)
    return
  }
  if (disposed) return
  mtime = file.mtime
  view = new EditorView({
    parent: el.value!,
    state: EditorState.create({
      doc: file.text,
      extensions: [
        // Переводы строк Windows сохраняем как были, иначе сохранение переписало бы весь файл
        file.text.includes('\r\n') ? EditorState.lineSeparator.of('\r\n') : [],
        basicSetup,
        original,
        changes,
        gutter({ class: 'cm-changes', markers: v => v.state.field(changes) }),
        language.of([]),
        appearance.of(look(isDark.value)),
        theme,
        Prec.high(keymap.of([{
          key: 'Mod-s',
          run: () => {
            save()
            return true
          },
        }])),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) dirty.value = !u.state.doc.eq(saved)
        }),
      ],
    }),
  })
  saved = view.state.doc
  if (props.active) view.focus()
  LanguageDescription.matchFilename(languages, props.path)?.load().then(support => view?.dispatch({ effects: language.reconfigure(support) }))
  check()
})

useIntervalFn(() => props.active && document.hasFocus() && check(), 2000)
useEventListener(window, 'focus', () => props.active && check())
watch(() => props.active, (active) => {
  if (!active) return
  check()
  nextTick(() => view?.focus())
})

onBeforeUnmount(() => {
  disposed = true
  view?.destroy()
  view = undefined
})
</script>

<template>
  <div class="flex size-full flex-col">
    <Alert v-if="conflict" class="mx-3 my-2 w-auto">
      <TriangleAlertIcon />
      <AlertTitle>Файл изменился на диске</AlertTitle>
      <AlertDescription class="flex flex-col gap-2">
        Скорее всего, его правит агент — а здесь есть несохранённые изменения.
        <div class="flex gap-2">
          <Button size="sm" variant="outline" @click="reload">
            Загрузить с диска
          </Button>
          <Button size="sm" variant="destructive" @click="save(true)">
            Перезаписать своими
          </Button>
        </div>
      </AlertDescription>
    </Alert>
    <Empty v-if="error" class="flex-1">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <FileWarningIcon />
        </EmptyMedia>
        <EmptyTitle>Не удалось открыть {{ baseName(path) }}</EmptyTitle>
        <EmptyDescription>{{ error }}</EmptyDescription>
      </EmptyHeader>
    </Empty>
    <div v-show="!error" ref="el" class="min-h-0 flex-1" :style="{ fontSize: `${settings.terminalFontSize}px` }" />
  </div>
</template>
