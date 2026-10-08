import assert from 'node:assert/strict'
import { test } from 'node:test'
import { comboOf, formatCombo } from '../app/utils/hotkeys.ts'

const key = (code: string, mods: Partial<Record<'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey', boolean>> = {}, k = code) =>
  ({ code, key: k, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...mods }) as KeyboardEvent

test('comboOf: по code, без модификатора и только с Shift — не хоткей', () => {
  assert.equal(comboOf(key('KeyK', { metaKey: true }, 'л')), 'Meta+KeyK')
  assert.equal(comboOf(key('KeyT', { metaKey: true, shiftKey: true })), 'Meta+Shift+KeyT')
  assert.equal(comboOf(key('KeyK')), undefined)
  assert.equal(comboOf(key('KeyK', { shiftKey: true })), undefined)
  assert.equal(comboOf(key('MetaLeft', { metaKey: true }, 'Meta')), undefined)
})

test('formatCombo: символы macOS', () => {
  assert.equal(formatCombo('Meta+Shift+KeyK'), '⌘⇧K')
  assert.equal(formatCombo('Meta+Comma'), '⌘,')
  assert.equal(formatCombo('Meta+Digit0'), '⌘0')
  assert.equal(formatCombo('Ctrl+Alt+BracketLeft'), '⌃⌥[')
})
