import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { recoveryKeyFor } from '../../services/auth/bundle'
import * as rsa from '../../services/cryptfns/rsa'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as wrapping from '../../services/cryptfns/wrapping'

import type { Authenticated } from '../../types'

interface CapturedSignatureRequest {
  fingerprint: string
  signature: string
  timestamp: number
  nonce: string
  remember: boolean
}

let signatureBody: Authenticated | null = null
let captured: CapturedSignatureRequest[] = []

vi.mock('../../services/api', () => ({
  default: {
    post: vi.fn(async (path: string, _query?: unknown, body?: unknown) => {
      if (path === '/api/auth/signature') {
        captured.push(body as CapturedSignatureRequest)
        return { body: signatureBody }
      }
      return { body: null }
    })
  },
  ErrorResponse: class {}
}))

function authenticatedFor(fingerprint: string): Authenticated {
  return {
    user: {
      id: 'u1',
      email: 'nonce@test.com',
      pubkey: 'PUB',
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

async function stores() {
  const { store: loginStore } = await import('../../services/auth/login')
  const { store: cryptoStore } = await import('../../services/crypto')
  return { login: loginStore(), crypto: cryptoStore() }
}

// The server rebuilds `fingerprint:timestamp:nonce` from the request fields
// and verifies the signature against that, so these tests pin the exact wire
// format the client must emit — a drift here is a login outage, not a
// cosmetic diff.
describe('signature login — client nonce', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    signatureBody = null
    captured = []
  })

  it('a curve account signs a random nonce + timestamp over the canonical', async () => {
    const edPriv = await ed25519.generatePrivateKey()
    const edPub = await ed25519.publicFromPrivate(edPriv)
    const xPriv = await wrapping.generatePrivateKey()
    const fingerprint = await ed25519.fingerprint(edPub)

    const material = recoveryKeyFor({
      keyType: 'curve25519',
      input: edPriv,
      wrappingPrivate: xPriv,
      legacyPrivate: null
    })

    const { login, crypto } = await stores()
    signatureBody = authenticatedFor(fingerprint)
    await login.withPrivateKey(crypto, { privateKey: material })

    expect(captured).toHaveLength(1)
    const req = captured[0]
    expect(req.fingerprint).toBe(fingerprint)
    expect(req.nonce).toMatch(/^[0-9a-f]{32}$/)
    expect(Math.abs(req.timestamp - Math.floor(Date.now() / 1000))).toBeLessThanOrEqual(5)

    const canonical = `${req.fingerprint}:${req.timestamp}:${req.nonce}`
    expect(await ed25519.verify(canonical, req.signature, edPub)).toBe(true)
  })

  it('an RSA account signs the same canonical', async () => {
    const kp = await rsa.generateKeyPair()

    const { login, crypto } = await stores()
    signatureBody = authenticatedFor(kp.fingerprint as string)
    await login.withPrivateKey(crypto, { privateKey: kp.input as string })

    expect(captured).toHaveLength(1)
    const req = captured[0]
    expect(req.nonce).toMatch(/^[0-9a-f]{32}$/)

    const canonical = `${req.fingerprint}:${req.timestamp}:${req.nonce}`
    expect(await rsa.verify(req.signature, canonical, kp.publicKey as string)).toBe(true)
  })

  it('back-to-back logins sign distinct nonces', async () => {
    const edPriv = await ed25519.generatePrivateKey()
    const edPub = await ed25519.publicFromPrivate(edPriv)
    const xPriv = await wrapping.generatePrivateKey()
    const fingerprint = await ed25519.fingerprint(edPub)

    const material = recoveryKeyFor({
      keyType: 'curve25519',
      input: edPriv,
      wrappingPrivate: xPriv,
      legacyPrivate: null
    })

    const { login, crypto } = await stores()
    signatureBody = authenticatedFor(fingerprint)
    await login.withPrivateKey(crypto, { privateKey: material })
    await login.withPrivateKey(crypto, { privateKey: material })

    expect(captured).toHaveLength(2)
    expect(captured[0].nonce).not.toBe(captured[1].nonce)
  })
})
