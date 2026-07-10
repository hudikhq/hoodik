import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { encodeBundle } from '../../services/auth/bundle'
import * as pk from '../../services/auth/pk'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as wrapping from '../../services/cryptfns/wrapping'

import type { Authenticated } from '../../types'

const DEVICE_ID = 'device-remember-me-test'

let refreshBody: Authenticated | null = null

vi.mock('../../services/api', () => ({
  default: {
    post: vi.fn(async (path: string) => {
      if (path === '/api/auth/refresh') {
        return { body: refreshBody }
      }
      return { body: null }
    })
  },
  ErrorResponse: class {}
}))

async function makeCurveAccount() {
  const edPriv = await ed25519.generatePrivateKey()
  const edPub = await ed25519.publicFromPrivate(edPriv)
  const xPriv = await wrapping.generatePrivateKey()
  const xPub = await wrapping.publicFromPrivate(xPriv)
  const fingerprint = await ed25519.fingerprint(edPub)
  return { edPriv, edPub, xPriv, xPub, fingerprint }
}

function authenticatedFor(fingerprint: string): Authenticated {
  return {
    user: {
      id: 'u1',
      email: 'curve@test.com',
      pubkey: 'EDPUB',
      fingerprint,
      created_at: 0,
      updated_at: 0,
      secret: false
    },
    session: {
      id: 's1',
      user_id: 'u1',
      device_id: DEVICE_ID,
      ip: '127.0.0.1',
      refresh: true,
      user_agent: 'test',
      created_at: 0,
      updated_at: 0,
      expires_at: Date.now() + 3_600_000
    }
  }
}

describe('Remember-me — curve25519 restore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    pk.clearRememberMe()
    refreshBody = null
  })

  it('UNIT: persisted curve bundle restores both identity and wrapping keys and verifies the fingerprint', async () => {
    const acct = await makeCurveAccount()

    // A curve account stores the whole bundle, not just the identity key.
    await pk.setRememberMe(
      encodeBundle({ identity: acct.edPriv, wrapping: acct.xPriv }),
      DEVICE_ID
    )

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    refreshBody = authenticatedFor(acct.fingerprint)
    await login.refresh(crypto)

    expect(crypto.keypair.keyType).toBe('curve25519')
    expect(crypto.keypair.input).toBe(acct.edPriv)
    expect(crypto.keypair.wrappingPrivate).toBe(acct.xPriv)
    expect(crypto.keypair.wrappingPublic).toBe(acct.xPub)
    expect(crypto.keypair.fingerprint).toBe(acct.fingerprint)
  })

  it('UNIT: a persisted bundle carrying the rsa segment restores the retained RSA key', async () => {
    const acct = await makeCurveAccount()
    const RSA_PEM = '-----BEGIN RSA PRIVATE KEY-----\nRETAINED\n-----END RSA PRIVATE KEY-----'

    await pk.setRememberMe(
      encodeBundle({ identity: acct.edPriv, wrapping: acct.xPriv, rsa: RSA_PEM }),
      DEVICE_ID
    )

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    refreshBody = authenticatedFor(acct.fingerprint)
    await login.refresh(crypto)

    // A migrated account's old RSA key rides inside the bundle so pre-migration
    // ciphertext stays readable after a browser reload.
    expect(crypto.keypair.legacyPrivate).toBe(RSA_PEM)
    expect(crypto.keypair.input).toBe(acct.edPriv)
    expect(crypto.keypair.wrappingPrivate).toBe(acct.xPriv)
  })

  it('UNIT: a bundle whose fingerprint does not match the authenticated user is rejected', async () => {
    const stored = await makeCurveAccount()
    const other = await makeCurveAccount()

    await pk.setRememberMe(
      encodeBundle({ identity: stored.edPriv, wrapping: stored.xPriv }),
      DEVICE_ID
    )

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    // The session claims a different account's fingerprint — restoring the
    // stored key would decrypt metadata with the wrong key, so refresh must
    // reject it instead of trusting the localStorage material.
    refreshBody = authenticatedFor(other.fingerprint)

    await expect(login.refresh(crypto)).rejects.toThrow(/No private key found/)
  })
})
