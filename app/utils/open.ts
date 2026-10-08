import { invoke } from '@tauri-apps/api/core'
import { toast } from 'vue-sonner'

/** Ссылка — в браузере по умолчанию (macOS `open`) */
export const openExternal = (url: string) =>
  invoke('open_path', { path: url }).catch(e => toast.error(String(e)))
