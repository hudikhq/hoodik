import { test } from 'node:test'
import assert from 'node:assert/strict'
import { findMatchOffsets } from './find-matches.ts'

test('empty query yields no matches', () => {
  assert.deepEqual(findMatchOffsets('hello world', ''), [])
})

test('finds a single case-insensitive match by default', () => {
  assert.deepEqual(findMatchOffsets('Hello World', 'hello'), [{ from: 0, to: 5 }])
})

test('case-sensitive search ignores different case', () => {
  assert.deepEqual(findMatchOffsets('Hello World', 'hello', true), [])
  assert.deepEqual(findMatchOffsets('Hello World', 'Hello', true), [{ from: 0, to: 5 }])
})

test('finds multiple non-overlapping matches', () => {
  assert.deepEqual(findMatchOffsets('aaa aaa', 'aa'), [
    { from: 0, to: 2 },
    { from: 4, to: 6 },
  ])
})

test('does not treat the query as a regex', () => {
  assert.deepEqual(findMatchOffsets('a.b aab', 'a.b'), [{ from: 0, to: 3 }])
  assert.deepEqual(findMatchOffsets('price is $5 (today)', '$5 (today)'), [
    { from: 9, to: 19 },
  ])
})

test('returns no matches when the needle is absent', () => {
  assert.deepEqual(findMatchOffsets('nothing here', 'zzz'), [])
})

test('finds a match at the end of the string', () => {
  assert.deepEqual(findMatchOffsets('xxend', 'end'), [{ from: 2, to: 5 }])
})
