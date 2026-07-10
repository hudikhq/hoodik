import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as cryptfns from '../../services/cryptfns'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as wrapping from '../../services/cryptfns/wrapping'
import * as envelope from '../../services/cryptfns/envelope'
import { encodeBundle } from '../../services/auth/bundle'
import * as opaque from '../../services/cryptfns/opaque'
import * as pk from '../../services/auth/pk'

const posts: Array<{ path: string; body: unknown }> = []

const SESSION = { device_id: 'dev1', expires_at: Date.now() + 3_600_000 }
const SERVER_KSF = { m_cost: 32 * 1024, t_cost: 4, p_cost: 2 }
const EXPORT_KEY = cryptfns.uint8.toBase64(new Uint8Array(32).fill(7))

let opaqueUser: Record<string, unknown> | null = null

vi.mock('../../services/api', () => ({
  default: {
    post: vi.fn(async (path: string, _query: unknown, body: unknown) => {
      posts.push({ path, body })
      if (path === '/api/auth/login/start') {
        return {
          body: {
            method: 'opaque',
            login_id: 'login-1',
            credential_response: 'srv-cred-resp',
            ksf: SERVER_KSF
          }
        }
      }
      if (path === '/api/auth/login/finish') {
        return { body: { user: opaqueUser, session: SESSION } }
      }
      return { body: null }
    })
  },
  ErrorResponse: class {}
}))

// The OPAQUE server half is not in the client WASM. The finish is a spy that
// returns the fixed export key the envelope was sealed under, so the real
// `_withOpaque` envelope/open/parse path runs end to end.
vi.mock('../../services/cryptfns/opaque', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../services/cryptfns/opaque')>()
  return {
    ...actual,
    clientLoginStart: vi.fn(async () => ({ message: 'login-req', state: 'login-st' })),
    clientLoginFinish: vi.fn(async () => ({
      finalization: 'fin',
      sessionKey: 'sk',
      exportKey: EXPORT_KEY
    }))
  }
})

describe('opaque login KSF threading', () => {
  beforeEach(async () => {
    posts.length = 0
    vi.mocked(opaque.clientLoginFinish).mockClear()
    pk.clearRememberMe()
    setActivePinia(createPinia())

    const edPriv = await ed25519.generatePrivateKey()
    const edPub = await ed25519.publicFromPrivate(edPriv)
    const xPriv = await wrapping.generatePrivateKey()
    const xPub = await wrapping.publicFromPrivate(xPriv)

    const kek = await envelope.deriveKek(cryptfns.uint8.fromBase64(EXPORT_KEY))
    const env = await envelope.seal(
      kek,
      new TextEncoder().encode(encodeBundle({ identity: edPriv, wrapping: xPriv }))
    )

    opaqueUser = {
      id: '00000000-0000-0000-0000-000000000009',
      email: 'migrated@test.com',
      pubkey: edPub,
      wrapping_pubkey: xPub,
      fingerprint: await ed25519.fingerprint(edPub),
      key_type: 'curve25519',
      encrypted_private_key: env,
      security_version: 1,
      secret: false,
      created_at: 0,
      updated_at: 0
    }
  })

  it('UNIT: login finish is stretched with the KSF login/start returned, not a constant', async () => {
    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    await login.withCredentials(crypto, { email: 'migrated@test.com', password: 'pw-123' })

    expect(vi.mocked(opaque.clientLoginFinish)).toHaveBeenCalledTimes(1)
    const [, , , ksf] = vi.mocked(opaque.clientLoginFinish).mock.calls[0]
    expect(ksf).toEqual(SERVER_KSF)

    // The session was established from the envelope opened with that export_key.
    expect(login.authenticated?.user.email).toBe('migrated@test.com')
    expect(crypto.keypair.keyType).toBe('curve25519')
  })
})
