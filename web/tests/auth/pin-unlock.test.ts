import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as pk from '../../services/auth/pk'
import { recoveryKeyFor } from '../../services/auth/bundle'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as wrapping from '../../services/cryptfns/wrapping'

import type { Authenticated } from '../../types'

const PIN = '1234'

let signatureBody: Authenticated | null = null

vi.mock('../../services/api', () => ({
  default: {
    post: vi.fn(async (path: string) => {
      if (path === '/api/auth/signature') {
        return { body: signatureBody }
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
      device_id: 'd1',
      ip: '127.0.0.1',
      refresh: false,
      user_agent: 'test',
      created_at: 0,
      updated_at: 0,
      expires_at: Date.now() + 3_600_000
    }
  }
}

describe('PIN unlock — curve25519', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    pk.clearPin()
    signatureBody = null
  })

  it('unlocks a migrated curve account: the PIN store holds the full bundle and restores both keys', async () => {
    const acct = await makeCurveAccount()

    // Mirror the lock-screen setup: it stores exactly what recoveryKeyFor produces.
    const material = recoveryKeyFor({
      keyType: 'curve25519',
      input: acct.edPriv,
      wrappingPrivate: acct.xPriv,
      legacyPrivate: null
    })
    await pk.pinEncryptAndStore(material, PIN, 'curve@test.com')

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    signatureBody = authenticatedFor(acct.fingerprint)
    await login.withPin(crypto, PIN)

    expect(crypto.keypair.keyType).toBe('curve25519')
    expect(crypto.keypair.input).toBe(acct.edPriv)
    expect(crypto.keypair.wrappingPrivate).toBe(acct.xPriv)
  })

  it('a PIN store holding only the identity key (the old write) is unrecoverable', async () => {
    const acct = await makeCurveAccount()

    // The pre-fix lock screen stored just keypair.input, dropping the wrapping
    // key. That material is neither a bundle nor an RSA PEM, so unlock cannot
    // rebuild the account — which is why the write side had to change too.
    await pk.pinEncryptAndStore(acct.edPriv, PIN, 'curve@test.com')

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    signatureBody = authenticatedFor(acct.fingerprint)
    await expect(login.withPin(crypto, PIN)).rejects.toThrow()
  })
})
