import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Состояние в ~/.diogen/state.json (Rust state_load/state_save): общее для dev-сборки и .app,
// у которых localStorage разный. Плагин persisted загружает файл до монтирования приложения
const refs = new Map<string, Ref<unknown>>()
let state: Record<string, unknown> | undefined

function hydrate(key: string, value: Ref<unknown>) {
  // Данные, сохранённые до переезда из localStorage, переносим один раз
  const legacy = localStorage.getItem(`diogen:${key}`)
  const known = key in state!
  if (known) value.value = state![key]
  else if (legacy) value.value = JSON.parse(legacy)
  watch(value, v => invoke('state_save', { key, value: v }), { deep: true, immediate: !known })
}

/** ref, который переживает перезапуск и виден и dev-сборке, и .app; можно объявлять на уровне модуля */
export function persisted<T>(key: string, fallback: T): Ref<T> {
  const value = ref(fallback) as Ref<T>
  refs.set(key, value)
  if (state) hydrate(key, value)
  return value
}

export async function loadPersisted() {
  state = await invoke<Record<string, unknown>>('state_load')
  refs.forEach((value, key) => hydrate(key, value))
}
