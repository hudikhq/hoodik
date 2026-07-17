import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as cryptfns from '../../services/cryptfns'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as wrapping from '../../services/cryptfns/wrapping'
import * as envelope from '../../services/cryptfns/envelope'
import * as pk from '../../services/auth/pk'
import { encodeBundle, parseBundle } from '../../services/auth/bundle'

const posts: Array<{ path: string; body: unknown }> = []

let signatureUser: Record<string, unknown> | null = null
const SESSION = { device_id: 'dev1', expires_at: Date.now() + 3_600_000 }

vi.mock('../../services/api', () => ({
  default: {
    post: vi.fn(async (path: string, _query: unknown, body: unknown) => {
      posts.push({ path, body })
      if (path === '/api/auth/signature') return { body: { user: signatureUser, session: SESSION } }
      if (path === '/api/auth/pake/register/start') {
        return { body: { registration_response: 'srv-reg-resp' } }
      }
      return { body: null }
    })
  },
  ErrorResponse: class {}
}))

const EXPORT_KEY = cryptfns.uint8.toBase64(new Uint8Array(32).fill(3))
vi.mock('../../services/cryptfns/opaque', () => ({
  clientRegistrationStart: vi.fn(async () => ({ state: 'reg-st', message: 'reg-req' })),
  clientRegistrationFinish: vi.fn(async () => ({ message: 'reg-upload', exportKey: EXPORT_KEY }))
}))

// The full Task 1 path: a migrated account backs up `v1|rsa:|ed:|x:`; the retained
// RSA key (which still decrypts pre-migration ciphertext) must survive the whole
// round trip — recovery-key login, the persisted remember-me material, the
// in-memory keypair, and a subsequent v2 password change.
describe('retained RSA key lifecycle', () => {
  beforeEach(() => {
    posts.length = 0
    signatureUser = null
    pk.clearRememberMe()
    setActivePinia(createPinia())
  })

  it('UNIT: survives recovery-key login (remember-me) and a later v2 password change', async () => {
    const edPriv = await ed25519.generatePrivateKey()
    const edPub = await ed25519.publicFromPrivate(edPriv)
    const xPriv = await wrapping.generatePrivateKey()
    const RSA_PEM = '-----BEGIN RSA PRIVATE KEY-----\nRETAINED-LEGACY\n-----END RSA PRIVATE KEY-----'
    const bundle = encodeBundle({ identity: edPriv, wrapping: xPriv, rsa: RSA_PEM })

    signatureUser = {
      id: 'u1',
      email: 'migrated@test.com',
      pubkey: edPub,
      fingerprint: await ed25519.fingerprint(edPub),
      security_version: 1,
      secret: false,
      created_at: 0,
      updated_at: 0
    }

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const { changePasswordV2 } = await import('../../services/account')
    const login = loginStore()
    const crypto = cryptoStore()

    await login.withPrivateKey(crypto, { privateKey: bundle, remember: true })

    // setupAndRemember persisted the rsa segment, and the in-memory keypair
    // carries it — neither drops the retained key.
    const remembered = await pk.getRememberMe('dev1')
    expect(parseBundle(remembered as string).rsa).toBe(RSA_PEM)
    expect(crypto.keypair.legacyPrivate).toBe(RSA_PEM)

    await changePasswordV2(crypto.keypair, 'correct horse battery staple v2')

    // The re-sealed envelope still carries the retained key after the change.
    const finish = posts.find((p) => p.path === '/api/auth/pake/register/finish')
    const reBody = finish!.body as { encrypted_private_key: string }
    const kek = await envelope.deriveKek(cryptfns.uint8.fromBase64(EXPORT_KEY))
    const reopened = await envelope.open(kek, reBody.encrypted_private_key)
    expect(parseBundle(new TextDecoder().decode(reopened)).rsa).toBe(RSA_PEM)
  })
})
