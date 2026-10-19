// port — строкой из формы; key — путь к приватному ключу (~ раскрывает ssh); args — доп. флаги как есть (-J, -L …)
export interface SshServer {
  id: string
  name: string
  host: string
  port?: string
  user?: string
  key?: string
  args?: string
}

/** Слово для shell: безопасные символы как есть, остальное — в одинарных кавычках */
export const shellQuote = (s: string) => /^[\w@%+=:,./~-]+$/.test(s) ? s : `'${s.replaceAll(`'`, `'\\''`)}'`

/**
 * Команда подключения для login-shell вкладки. С сохранённым паролем его подставит $VIBECRAFT_DIR/askpass.sh
 * из credentials.json — в команде и истории shell только id записи
 */
export function sshCommand(s: SshServer, password: boolean) {
  const words = ['ssh']
  if (s.port?.trim()) words.push('-p', shellQuote(s.port.trim()))
  // IdentitiesOnly: без него ssh сначала переберёт ключи агента и сервер может отказать по числу попыток
  if (s.key?.trim()) words.push('-i', shellQuote(s.key.trim()), '-o', 'IdentitiesOnly=yes')
  if (s.args?.trim()) words.push(s.args.trim())
  words.push(shellQuote(s.user?.trim() ? `${s.user.trim()}@${s.host.trim()}` : s.host.trim()))
  const env = password ? `VIBECRAFT_SSH=ssh-${s.id} SSH_ASKPASS="$VIBECRAFT_DIR/askpass.sh" SSH_ASKPASS_REQUIRE=force ` : ''
  return env + words.join(' ')
}

/** Псевдонимы из ~/.ssh/config без шаблонов (*, ?, !). ponytail: Include не разбираем — добавить, если попросят */
export const configHosts = (config: string) => [...new Set(config
  .split('\n')
  .flatMap(line => line.trim().match(/^Host(?:\s+|\s*=\s*)(.+)/i)?.[1]?.split(/\s+/) ?? [])
  .filter(h => !/[*?!]/.test(h)))]
