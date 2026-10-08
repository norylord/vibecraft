export interface DiffFile { path: string, lines: string[], added: number, removed: number }

/** Unified diff (`git diff`) → файлы со строками ханков и счётчиками +/− */
export function parseDiff(diff: string): DiffFile[] {
  return diff
    .split(/^(?=diff --git )/m)
    .filter(chunk => chunk.startsWith('diff --git '))
    .map((chunk) => {
      const all = chunk.trimEnd().split('\n')
      const to = all.find(l => l.startsWith('+++ '))?.slice(4)
      const from = all.find(l => l.startsWith('--- '))?.slice(4)
      // Удалённый файл: +++ /dev/null — путь берём из ---; без них (бинарник) — из заголовка
      const path = (to && to !== '/dev/null' ? to : from)?.replace(/^[ab]\//, '')
        ?? all[0]!.replace(/^diff --git a\/.* b\//, '')
      const hunk = all.findIndex(l => l.startsWith('@@'))
      // Нет ханков (бинарник, переименование, смена режима) — показываем служебные строки
      const lines = hunk === -1 ? all.slice(1) : all.slice(hunk)
      return {
        path,
        lines,
        added: lines.filter(l => l.startsWith('+')).length,
        removed: lines.filter(l => l.startsWith('-')).length,
      }
    })
}
