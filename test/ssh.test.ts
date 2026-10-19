import assert from 'node:assert/strict'
import { test } from 'node:test'
import { configHosts, sshCommand } from '../app/utils/ssh.ts'

test('sshCommand: порт, ключ, флаги, кавычки, пароль через askpass', () => {
  assert.equal(sshCommand({ id: '1', name: '', host: 'example.com' }, false), 'ssh example.com')
  assert.equal(
    sshCommand({ id: '1', name: '', host: ' 10.0.0.1 ', port: '2222', user: 'root', key: '~/.ssh/my key', args: '-J bastion' }, false),
    `ssh -p 2222 -i '~/.ssh/my key' -o IdentitiesOnly=yes -J bastion root@10.0.0.1`,
  )
  assert.equal(sshCommand({ id: '1', name: '', host: 'h', user: `o'neil` }, false), `ssh 'o'\\''neil@h'`)
  assert.equal(
    sshCommand({ id: 'ab', name: '', host: 'h' }, true),
    'VIBECRAFT_SSH=ssh-ab SSH_ASKPASS="$VIBECRAFT_DIR/askpass.sh" SSH_ASKPASS_REQUIRE=force ssh h',
  )
})

test('configHosts: псевдонимы без шаблонов и дублей', () => {
  const config = `
Host *
  ServerAliveInterval 30
Host prod prod-db
  HostName 10.0.0.1
  # Host commented
host=staging
Host !bad web-? prod
Match host x
`
  assert.deepEqual(configHosts(config), ['prod', 'prod-db', 'staging'])
})
