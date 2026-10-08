// pnpm test — Node 24 запускает TS без сборки
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { parseDiff } from '../app/utils/diff.ts'

test('parseDiff: изменённый, новый, удалённый и бинарный файлы', () => {
  const diff = [
    'diff --git a/src/a.ts b/src/a.ts',
    'index 1..2 100644',
    '--- a/src/a.ts',
    '+++ b/src/a.ts',
    '@@ -1,2 +1,2 @@',
    ' keep',
    '-old',
    '+new',
    'diff --git a/папка/новый.md b/папка/новый.md',
    'new file mode 100644',
    '--- /dev/null',
    '+++ b/папка/новый.md',
    '@@ -0,0 +1 @@',
    '+hello',
    'diff --git a/gone.txt b/gone.txt',
    'deleted file mode 100644',
    '--- a/gone.txt',
    '+++ /dev/null',
    '@@ -1 +0,0 @@',
    '-bye',
    'diff --git a/logo.png b/logo.png',
    'Binary files a/logo.png and b/logo.png differ',
    '',
  ].join('\n')

  assert.deepEqual(parseDiff(diff).map(f => [f.path, f.added, f.removed]), [
    ['src/a.ts', 1, 1],
    ['папка/новый.md', 1, 0],
    ['gone.txt', 0, 1],
    ['logo.png', 0, 0],
  ])
  assert.deepEqual(parseDiff(diff)[3]!.lines, ['Binary files a/logo.png and b/logo.png differ'])
  assert.deepEqual(parseDiff(''), [])
})
