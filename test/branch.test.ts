import assert from 'node:assert/strict'
import { test } from 'node:test'
import { branchName } from '../app/utils/branch.ts'

test('branchName: транслит, мусор, пустой slug, обрезка', () => {
  assert.equal(branchName('DIO-12', 'Статусы агентов!'), 'DIO-12-statusy-agentov')
  assert.equal(branchName('DIO-7', 'Fix: «MR» panel / v2'), 'DIO-7-fix-mr-panel-v2')
  assert.equal(branchName('DIO-1', '???'), 'DIO-1')
  assert.ok(!branchName('DIO-3', 'очень длинное название задачи которое не влезет в ветку').endsWith('-'))
})
