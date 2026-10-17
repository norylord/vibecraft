/**
 * Хвост лога job'а для агента: причина падения обычно в конце. Без ANSI-цветов, маркеров секций GitLab
 * (`section_start:…\r\x1B[0K`) и управляющих символов; из строки с `\r` — то, что осталось на экране
 */
export function jobLogTail(trace: string, lines = 150) {
  return trace
    .split('\n')
    .map(line => line.replace(/\r$/, '').split('\r').at(-1)!.replace(/\x1B\[[0-9;?]*[A-Za-z~]/g, '').replace(/[\x00-\x08\x0B-\x1F\x7F]/g, ''))
    .slice(-lines)
    .join('\n')
    .trim()
}
