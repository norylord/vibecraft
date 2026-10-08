const rtf = new Intl.RelativeTimeFormat('ru', { numeric: 'auto' })
const UNITS = [['year', 31536000], ['month', 2592000], ['day', 86400], ['hour', 3600], ['minute', 60]] as const

/** «2 часа назад», «позавчера»; принимает мс или ISO-строку */
export function ago(time: number | string) {
  const seconds = (new Date(time).getTime() - Date.now()) / 1000
  const unit = UNITS.find(([, s]) => Math.abs(seconds) >= s)
  return unit ? rtf.format(Math.round(seconds / unit[1]), unit[0]) : 'только что'
}
