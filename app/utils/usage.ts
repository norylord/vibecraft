// Окно лимита подписки агента: сколько израсходовано и когда сбросится
export interface UsageWindow { label: string, percent: number, resetsAt?: number }

// Claude (/api/oauth/usage): utilization в процентах, resets_at — ISO-строка
export function claudeWindows(raw: any): UsageWindow[] {
  return ([['five_hour', '5 ч'], ['seven_day', 'Неделя']] as const)
    .filter(([key]) => typeof raw?.[key]?.utilization === 'number')
    .map(([key, label]) => ({ label, percent: raw[key].utilization, resetsAt: raw[key].resets_at ? Date.parse(raw[key].resets_at) : undefined }))
}

const windowLabel = (mins: number) =>
  mins < 1440 ? `${Math.round(mins / 60)} ч` : mins === 10080 ? 'Неделя' : mins === 43200 ? 'Месяц' : `${Math.round(mins / 1440)} дн`

// Codex (account/rateLimits/read): окна primary/secondary, длительность в минутах, resetsAt — секунды
export function codexWindows(raw: any): UsageWindow[] {
  return [raw?.primary, raw?.secondary]
    .filter(w => typeof w?.usedPercent === 'number')
    .map(w => ({
      label: w.windowDurationMins ? windowLabel(w.windowDurationMins) : 'Лимит',
      percent: w.usedPercent,
      resetsAt: w.resetsAt ? w.resetsAt * 1000 : undefined,
    }))
}
