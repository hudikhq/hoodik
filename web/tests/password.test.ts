import { describe, expect, it } from 'vitest'

import { isStrongPassword } from '../src/utils/password'

describe('password validation', () => {
  it('requires zxcvbn score above 3 to match backend validation', () => {
    expect(isStrongPassword('weak-pass')).toBe(false)
    expect(isStrongPassword('strong-password-123!')).toBe(true)
  })
})
