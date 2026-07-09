import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { encodeBundle, parseBundle } from '../../services/auth/bundle'
import * as cryptfns from '../../services/cryptfns'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as x25519 from '../../services/cryptfns/x25519'
import * as envelope from '../../services/cryptfns/envelope'

import type { CreateUser } from '../../types'

const posts: Array<{ path: string; body: unknown }> = []

vi.mock('../../services/api', () => {
  return {
    default: {
      post: vi.fn(async (path: string, _query: unknown, body: unknown) => {
        posts.push({ path, body })
        if (path === '/api/auth/register/pake/start') {
          return { body: { registration_response: 'server-registration-response' } }
        }
        if (path === '/api/auth/register') {
          return {
            body: {
              user: { id: 'u1', email: 'ceremony@test.com', pubkey: '', fingerprint: '' },
              session: { device_id: 'd1', expires_at: null }
            }
          }
        }
        return { body: null }
      })
    },
    ErrorResponse: class {}
  }
})

// The server half of OPAQUE (registration_response generation) is not in the
// client WASM, so the finish step is stubbed with a real 32-byte export key —
// enough to drive envelope seal/open deterministically.
const EXPORT_KEY = cryptfns.uint8.toBase64(new Uint8Array(32).fill(7))
vi.mock('../../services/cryptfns/opaque', () => ({
  clientRegistrationStart: vi.fn(async () => ({ state: 'st', message: 'client-request' })),
  clientRegistrationFinish: vi.fn(async () => ({
    message: 'client-upload',
    exportKey: EXPORT_KEY
  }))
}))

describe('Bundle encode/parse', () => {
  it('UNIT: round-trips a fresh (ed+x) bundle', () => {
    const encoded = encodeBundle({ identity: 'ED_PEM', wrapping: 'X_PEM' })
    expect(encoded).toBe('v1|ed:ED_PEM|x:X_PEM')
    expect(parseBundle(encoded)).toEqual({ identity: 'ED_PEM', wrapping: 'X_PEM', rsa: undefined })
  })

  it('UNIT: round-trips a migrated (rsa+ed+x) bundle', () => {
    const encoded = encodeBundle({ identity: 'ED', wrapping: 'X', rsa: 'RSA' })
    expect(encoded).toBe('v1|rsa:RSA|ed:ED|x:X')
    expect(parseBundle(encoded)).toEqual({ identity: 'ED', wrapping: 'X', rsa: 'RSA' })
  })
})

describe('Register ceremony self-check crypto', () => {
  it('UNIT: sealed bundle reopens and the identity key signs', async () => {
    const edPriv = await ed25519.generatePrivateKey()
    const edPub = await ed25519.publicFromPrivate(edPriv)
    const xPriv = await x25519.generatePrivateKey()

    const kek = await envelope.deriveKek(cryptfns.uint8.fromBase64(EXPORT_KEY))
    const sealed = await envelope.seal(
      kek,
      new TextEncoder().encode(encodeBundle({ identity: edPriv, wrapping: xPriv }))
    )
    const reopened = await envelope.open(kek, sealed)
    expect(reopened.length).toBeGreaterThan(0)

    const parsed = parseBundle(new TextDecoder().decode(reopened))
    expect(parsed.identity).toBe(edPriv)
    expect(parsed.wrapping).toBe(xPriv)

    const sig = await ed25519.sign('probe', edPriv)
    expect(await ed25519.verify('probe', sig, edPub)).toBe(true)
  })
})

describe('Register store — request body', () => {
  beforeEach(() => {
    posts.length = 0
    setActivePinia(createPinia())
  })

  it('UNIT: POSTs the exact v2 field names the server DTO requires', async () => {
    const { store: registerStore } = await import('../../services/auth/register')
    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')

    const register = registerStore()
    const login = loginStore()
    const crypto = cryptoStore()

    const edPriv = await ed25519.generatePrivateKey()
    const edPub = await ed25519.publicFromPrivate(edPriv)
    const xPriv = await x25519.generatePrivateKey()
    const xPub = await x25519.publicFromPrivate(xPriv)

    const data: CreateUser = {
      email: 'ceremony@test.com',
      password: 'correct horse battery staple',
      pubkey: edPub,
      wrapping_pubkey: xPub,
      fingerprint: await ed25519.fingerprint(edPub),
      identity_private_key: edPriv,
      wrapping_private_key: xPriv
    }

    await register.register(data, login, crypto)

    const startPost = posts.find((p) => p.path === '/api/auth/register/pake/start')
    expect(startPost).toBeDefined()
    expect(Object.keys(startPost!.body as object).sort()).toEqual(['email', 'registration_request'])
    expect((startPost!.body as Record<string, unknown>).email).toBe('ceremony@test.com')

    const registerPost = posts.find((p) => p.path === '/api/auth/register')
    expect(registerPost).toBeDefined()
    const body = registerPost!.body as Record<string, unknown>

    // Exact DTO field set — a mismatch 422s silently against
    // auth/src/data/create_user.rs.
    expect(Object.keys(body).sort()).toEqual(
      [
        'email',
        'encrypted_private_key',
        'fingerprint',
        'invitation_id',
        'key_type',
        'opaque_registration_upload',
        'pubkey',
        'secret',
        'token',
        'wrapping_pubkey'
      ].sort()
    )
    expect(body).not.toHaveProperty('password')
    expect(body.key_type).toBe('curve25519')
    expect(body.pubkey).toBe(edPub)
    expect(body.wrapping_pubkey).toBe(xPub)
    expect(body.fingerprint).toBe(data.fingerprint)
    expect(typeof body.encrypted_private_key).toBe('string')
    expect(body.opaque_registration_upload).toBe('client-upload')
  })
})
