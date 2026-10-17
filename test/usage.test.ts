import assert from 'node:assert/strict'
import { test } from 'node:test'
import { claudeWindows, codexWindows } from '../app/utils/usage.ts'

test('claudeWindows: 5 ч и неделя, пустые окна пропускаются', () => {
  assert.deepEqual(claudeWindows({
    five_hour: { utilization: 23.5, resets_at: '2026-10-14T12:00:00+00:00' },
    seven_day: { utilization: 41, resets_at: null },
    seven_day_opus: null,
  }), [
    { label: '5 ч', percent: 23.5, resetsAt: Date.parse('2026-10-14T12:00:00Z') },
    { label: 'Неделя', percent: 41, resetsAt: undefined },
  ])
  assert.deepEqual(claudeWindows({ five_hour: null }), [])
})

test('codexWindows: подписи по длительности окна, секунды → мс', () => {
  assert.deepEqual(codexWindows({
    primary: { usedPercent: 12, windowDurationMins: 300, resetsAt: 1794585124 },
    secondary: { usedPercent: 30, windowDurationMins: 10080, resetsAt: null },
  }), [
    { label: '5 ч', percent: 12, resetsAt: 1794585124000 },
    { label: 'Неделя', percent: 30, resetsAt: undefined },
  ])
  assert.deepEqual(codexWindows({ primary: { usedPercent: 0, windowDurationMins: 43200, resetsAt: null }, secondary: null }).map(w => w.label), ['Месяц'])
  assert.deepEqual(codexWindows(null), [])
})
