import assert from 'node:assert/strict'
import { test } from 'node:test'
import { checksStatus, commitUrl, forgeOf, parseRemote } from '../app/utils/forge.ts'

test('parseRemote: ssh, https, ssh:// с портом, токен в URL, не URL', () => {
  assert.deepEqual(parseRemote('git@gitlab.lince.studio:web/shop.git'), { host: 'gitlab.lince.studio', path: 'web/shop', url: 'https://gitlab.lince.studio/web/shop' })
  assert.equal(parseRemote('https://gitlab.lince.studio/web/sub/shop.git')?.path, 'web/sub/shop')
  assert.equal(parseRemote('https://oauth2:tok@gitlab.lince.studio/web/shop')?.url, 'https://gitlab.lince.studio/web/shop')
  assert.equal(parseRemote('ssh://git@gitlab.lince.studio:2222/web/shop.git')?.url, 'https://gitlab.lince.studio/web/shop')
  assert.equal(parseRemote('http://git.local:8080/team/app.git')?.url, 'http://git.local:8080/team/app')
  assert.equal(parseRemote('not a url'), undefined)
  assert.equal(parseRemote('/srv/git/app.git'), undefined)
  assert.equal(parseRemote('file:///srv/git/app.git'), undefined)
})

test('forgeOf: известные хосты, свой GitLab и GitHub Enterprise из настроек, API', () => {
  const gl = 'https://gitlab.lince.studio'
  assert.deepEqual(forgeOf('git@github.com:norylord/vibecraft.git', {}), {
    kind: 'github', name: 'GitHub', host: 'github.com', path: 'norylord/vibecraft', url: 'https://github.com/norylord/vibecraft', api: true,
  })
  assert.equal(forgeOf('git@gitlab.lince.studio:web/shop.git', { gitlab: gl })?.kind, 'gitlab')
  assert.equal(forgeOf('git@gitlab.lince.studio:web/shop.git', { gitlab: gl })?.api, true)
  assert.equal(forgeOf('git@gitlab.lince.studio:web/shop.git', {})?.kind, 'other', 'свой GitLab без интеграции не угадываем')
  assert.equal(forgeOf('https://gitlab.com/a/b.git', {})?.api, false, 'gitlab.com без токена — только ссылка')
  assert.equal(forgeOf('https://bitbucket.org/a/b.git', {})?.name, 'Bitbucket')
  assert.equal(forgeOf('git@git.example.com:a/b.git', {})?.name, 'git.example.com')
  const ghe = forgeOf('git@ghe.corp:team/app.git', { github: 'https://ghe.corp' })
  assert.equal(ghe?.kind, 'github')
  assert.equal(ghe?.api, true)
  assert.equal(forgeOf('git@github.com:a/b.git', { github: 'https://api.github.com' })?.api, true)
  assert.equal(forgeOf('git@github.com:a/b.git', { github: 'https://ghe.corp' })?.api, false, 'API настроен на другой хост')
})

test('commitUrl: по хостингу; у неизвестного ссылки нет', () => {
  const gh = forgeOf('git@github.com:a/b.git', {})!
  assert.equal(commitUrl(gh, 'abc'), 'https://github.com/a/b/commit/abc')
  assert.equal(commitUrl(forgeOf('https://gitlab.com/g/p.git', {})!, 'abc'), 'https://gitlab.com/g/p/-/commit/abc')
  assert.equal(commitUrl(forgeOf('git@git.example.com:a/b.git', {})!, 'abc'), undefined)
})

test('checksStatus: идут, упали, отменены, пропущены, прошли, нет проверок', () => {
  const run = (status: string, conclusion: string | null = null) => ({ status, conclusion })
  assert.equal(checksStatus([]), undefined)
  assert.equal(checksStatus([run('completed', 'success'), run('in_progress')]), 'running')
  assert.equal(checksStatus([run('completed', 'success'), run('completed', 'timed_out')]), 'failed')
  assert.equal(checksStatus([run('completed', 'success'), run('completed', 'cancelled')]), 'canceled')
  assert.equal(checksStatus([run('completed', 'skipped'), run('completed', 'neutral')]), 'skipped')
  assert.equal(checksStatus([run('completed', 'success'), run('completed', 'skipped')]), 'success')
})
