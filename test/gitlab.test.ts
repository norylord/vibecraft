import assert from 'node:assert/strict'
import { test } from 'node:test'
import { jobLogTail } from '../app/utils/gitlab.ts'

test('jobLogTail: цвета, секции GitLab, перезапись строки через \\r, хвост', () => {
  const trace = 'section_start:1700000000:step_script\r\x1B[0K\x1B[36;1mExecuting\x1B[0;m\r\n'
    + 'progress 10%\rprogress 100%\n'
    + '\x1B[31mError: test failed\x1B[0m\x1B[201~\n'
    + 'section_end:1700000001:step_script\r\x1B[0K\n'
  assert.equal(jobLogTail(trace), 'Executing\nprogress 100%\nError: test failed')
  assert.equal(jobLogTail('a\nb\nc', 2), 'b\nc')
})
