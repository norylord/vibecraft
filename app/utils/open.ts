import { invoke } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'

/** Ссылка — в браузере по умолчанию (macOS `open`) */
export const openExternal = (url: string) =>
  invoke('open_path', { path: url }).catch(e => toast.error(String(e)))

/** Путь — в приложении (`open -a`), без app — в Finder; reveal — выделить в его папке в Finder */
export const openPath = (path: string, opts: { app?: string, reveal?: boolean } = {}) =>
  invoke('open_path', { path, ...opts }).catch(e => toast.error(String(e)))
