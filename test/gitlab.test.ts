import assert from 'node:assert/strict'
import { test } from 'node:test'
import { gitlabProject } from '../app/utils/gitlab.ts'

test('gitlabProject: ssh, https, ssh:// с портом, чужой хост', () => {
  const gl = 'https://gitlab.lince.studio'
  assert.equal(gitlabProject('git@gitlab.lince.studio:web/shop.git', gl), 'web/shop')
  assert.equal(gitlabProject('https://gitlab.lince.studio/web/sub/shop.git', gl), 'web/sub/shop')
  assert.equal(gitlabProject('https://oauth2:tok@gitlab.lince.studio/web/shop', gl), 'web/shop')
  assert.equal(gitlabProject('ssh://git@gitlab.lince.studio:2222/web/shop.git', gl), 'web/shop')
  assert.equal(gitlabProject('https://github.com/norylord/vibecraft.git', gl), undefined)
  assert.equal(gitlabProject('not a url', gl), undefined)
})
