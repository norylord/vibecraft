import type { Update } from '@tauri-apps/plugin-updater'
import { invoke } from '@tauri-apps/api/core'
import { check } from '@tauri-apps/plugin-updater'
import { toast } from 'vue-sonner'

// Найденное обновление: version, body — список изменений из релиза (latest.json на GitHub)
// ponytail: при пропуске нескольких версий виден список только последней — брать все релизы из GitHub API, если понадобится
const update = shallowRef<Update>()
// Диалог «Что нового»
const open = ref(false)
const checking = ref(false)
// Скачано 0..1; undefined — не качаем
const progress = ref<number>()

export function useUpdater() {
  // manual — из настроек: сообщаем и «обновлений нет», и ошибку; фоновая проверка молчит
  async function checkUpdate(manual = false) {
    if (checking.value || progress.value !== undefined) return
    checking.value = true
    try {
      const found = await check()
      if (!found) {
        if (manual) toast.success('Установлена последняя версия')
        return
      }
      const known = found.version === update.value?.version
      await update.value?.close()
      update.value = found
      if (manual) open.value = true
      else if (!known) toast.info(`Доступна версия ${found.version}`, { duration: Infinity, action: { label: 'Что нового', onClick: () => (open.value = true) } })
    }
    catch (e) {
      if (manual) toast.error(`Не удалось проверить обновления: ${e}`)
    }
    finally {
      checking.value = false
    }
  }

  async function install() {
    let total = 0
    let done = 0
    progress.value = 0
    try {
      await update.value!.downloadAndInstall((e) => {
        if (e.event === 'Started') total = e.data.contentLength ?? 0
        else if (e.event === 'Progress' && total) progress.value = (done += e.data.chunkLength) / total
      })
      await invoke('restart')
    }
    catch (e) {
      progress.value = undefined
      toast.error(`Не удалось обновить: ${e}`)
    }
  }

  return { update, open, checking, progress, checkUpdate, install }
}
