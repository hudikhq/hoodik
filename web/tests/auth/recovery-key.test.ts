import { describe, it, expect, beforeEach } from 'vitest'

import { recoveryKeyFor, parseBundle } from '../../services/auth/bundle'
import * as migrationNotice from '../../services/auth/migration-notice'

describe('recoveryKeyFor', () => {
  it('encodes the full bundle for a migrated curve account, retaining the RSA key', () => {
    const key = recoveryKeyFor({
      keyType: 'curve25519',
      input: 'ED-PRIV',
      wrappingPrivate: 'X-PRIV',
      legacyPrivate: 'RSA-PRIV'
    })
    const parsed = parseBundle(key)
    expect(parsed.identity).toBe('ED-PRIV')
    expect(parsed.wrapping).toBe('X-PRIV')
    expect(parsed.rsa).toBe('RSA-PRIV')
  })

  it('omits the RSA segment for a natively-registered curve account', () => {
    const key = recoveryKeyFor({
      keyType: 'curve25519',
      input: 'ED-PRIV',
      wrappingPrivate: 'X-PRIV',
      legacyPrivate: null
    })
    expect(key.includes('rsa:')).toBe(false)
    expect(parseBundle(key).rsa).toBeUndefined()
  })

  it('returns the raw RSA PEM for a legacy account', () => {
    const key = recoveryKeyFor({ keyType: 'rsa', input: 'RSA-PEM' })
    expect(key).toBe('RSA-PEM')
  })
})

describe('migration notice flag', () => {
  const user = 'user-abc'
  beforeEach(() => localStorage.clear())

  it('is not pending until the ceremony marks it', () => {
    expect(migrationNotice.isPending(user)).toBe(false)
  })

  it('is pending after marking, and clears on acknowledge', () => {
    migrationNotice.markPending(user)
    expect(migrationNotice.isPending(user)).toBe(true)
    migrationNotice.acknowledge(user)
    expect(migrationNotice.isPending(user)).toBe(false)
  })

  it('is scoped per user', () => {
    migrationNotice.markPending(user)
    expect(migrationNotice.isPending('someone-else')).toBe(false)
  })
})
