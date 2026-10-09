import assert from 'node:assert/strict'
import { test } from 'node:test'
import { gitlabProject, jobLogTail } from '../app/utils/gitlab.ts'

test('gitlabProject: ssh, https, ssh:// с портом, чужой хост', () => {
  const gl = 'https://gitlab.lince.studio'
  assert.equal(gitlabProject('git@gitlab.lince.studio:web/shop.git', gl), 'web/shop')
  assert.equal(gitlabProject('https://gitlab.lince.studio/web/sub/shop.git', gl), 'web/sub/shop')
  assert.equal(gitlabProject('https://oauth2:tok@gitlab.lince.studio/web/shop', gl), 'web/shop')
  assert.equal(gitlabProject('ssh://git@gitlab.lince.studio:2222/web/shop.git', gl), 'web/shop')
  assert.equal(gitlabProject('https://github.com/norylord/vibecraft.git', gl), undefined)
  assert.equal(gitlabProject('not a url', gl), undefined)
})

test('jobLogTail: цвета, секции GitLab, перезапись строки через \\r, хвост', () => {
  const trace = 'section_start:1700000000:step_script\r\x1B[0K\x1B[36;1mExecuting\x1B[0;m\r\n'
    + 'progress 10%\rprogress 100%\n'
    + '\x1B[31mError: test failed\x1B[0m\x1B[201~\n'
    + 'section_end:1700000001:step_script\r\x1B[0K\n'
  assert.equal(jobLogTail(trace), 'Executing\nprogress 100%\nError: test failed')
  assert.equal(jobLogTail('a\nb\nc', 2), 'b\nc')
})
