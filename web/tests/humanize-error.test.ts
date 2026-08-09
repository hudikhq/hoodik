import { describe, it, expect } from 'vitest'
import { humanizeError } from '../services'

describe('humanizeError', () => {
  it('names a network failure instead of passing the browser wording through', () => {
    // fetch rejects with a TypeError whose message differs per browser:
    // "Failed to fetch" (Chrome), "Load failed" (Safari), "NetworkError…"
    for (const message of ['Failed to fetch', 'Load failed', 'NetworkError when attempting to fetch']) {
      expect(humanizeError(new TypeError(message))).not.toContain(message)
      expect(humanizeError(new TypeError(message))).toMatch(/reach the server/i)
    }
  })

  it('prefers the API description when the server explained itself', () => {
    const apiError = { kind: 'ErrorResponse', description: 'A file with this name already exists' }

    expect(humanizeError(apiError)).toBe('A file with this name already exists')
  })

  it('keeps a meaningful application message', () => {
    expect(humanizeError(new Error('Upload has not finished'))).toBe('Upload has not finished')
  })

  it('never returns an empty string', () => {
    for (const value of [undefined, null, {}, new Error(''), '']) {
      expect(humanizeError(value)).toBeTruthy()
    }
  })
})
