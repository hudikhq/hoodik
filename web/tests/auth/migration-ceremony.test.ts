import { describe, it, expect, beforeAll, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as cryptfns from '../../services/cryptfns'
import * as wrapping from '../../services/cryptfns/wrapping'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as transition from '../../services/cryptfns/transition'
import * as pk from '../../services/auth/pk'
import { parseBundle } from '../../services/auth/bundle'
import { notify } from '@kyvg/vue3-notification'

import type { KeyPair } from '../../types'

const posts: Array<{ path: string; body: unknown }> = []

let loginUser: Record<string, unknown> | null = null
let refreshUser: Record<string, unknown> | null = null
let migrationKeys: {
  keys: Array<{ file_id: string; encrypted_key: string }>
  link_keys: Array<{ link_id: string; encrypted_link_key: string; file_id: string }>
} = { keys: [], link_keys: [] }

vi.mock('@kyvg/vue3-notification', () => ({ notify: vi.fn() }))

const SESSION = { device_id: 'dev1', expires_at: Date.now() + 3_600_000 }

vi.mock('../../services/api', () => ({
  default: {
    get: vi.fn(async (path: string) => {
      // The client appends ?offset=&limit= to page the key set; a single page
      // (next_offset null) is enough for these small fixtures.
      if (path.startsWith('/api/auth/migration/keys')) {
        return {
          body: {
            keys: migrationKeys.keys,
            link_keys: migrationKeys.link_keys,
            next_offset: null
          }
        }
      }
      return { body: null }
    }),
    post: vi.fn(async (path: string, _query: unknown, body: unknown) => {
      posts.push({ path, body })
      if (path === '/api/auth/login/start') return { body: { method: 'password' } }
      // The store strips device_id off the session it is handed, so each
      // response gets its own copy to keep SESSION intact across tests.
      if (path === '/api/auth/login') return { body: { user: loginUser, session: { ...SESSION } } }
      if (path === '/api/auth/signature') {
        return { body: { user: loginUser, session: { ...SESSION } } }
      }
      if (path === '/api/auth/refresh') {
        return { body: refreshUser ? { user: refreshUser, session: { ...SESSION } } : null }
      }
      if (path === '/api/auth/pake/register/start') {
        return { body: { registration_response: 'srv-reg-resp' } }
      }
      if (path === '/api/auth/migration/complete') return { body: { ok: true } }
      return { body: null }
    })
  },
  ErrorResponse: class {}
}))

// The OPAQUE server half isn't in the client WASM; stub it with a real 32-byte
// export key so the envelope seal/open runs deterministically end to end.
const EXPORT_KEY = cryptfns.uint8.toBase64(new Uint8Array(32).fill(5))
vi.mock('../../services/cryptfns/opaque', () => ({
  clientLoginStart: vi.fn(async () => ({ message: 'login-req', state: 'login-st' })),
  clientLoginFinish: vi.fn(async () => ({ finalization: 'fin', exportKey: EXPORT_KEY })),
  clientRegistrationStart: vi.fn(async () => ({ state: 'reg-st', message: 'reg-req' })),
  clientRegistrationFinish: vi.fn(async () => ({ message: 'reg-upload', exportKey: EXPORT_KEY }))
}))

// The transition certificate and audit-event signing are covered by their own
// crate tests, and the audit binding is absent from the client WASM; both are
// stubbed so the suite stays focused on the re-wrap contract. The audit stub
// records its arguments so a test can pin the canonical inputs.
vi.mock('../../services/cryptfns/transition', () => ({
  sign: vi.fn(async () => ({ oldSignature: 'old-sig', newSignature: 'new-sig' })),
  keyRotationAuditSign: vi.fn(async () => 'audit-sig')
}))

const PASSWORD = 'legacy-password-123'
let rsaKp: KeyPair
let encPriv: string

beforeAll(async () => {
  rsaKp = await cryptfns.rsa.generateKeyPair()
  encPriv = await cryptfns.rsa.protectPrivateKey(rsaKp.input as string, PASSWORD)
})

function legacyUser() {
  return {
    id: '00000000-0000-0000-0000-000000000001',
    email: 'legacy@test.com',
    pubkey: rsaKp.publicKey,
    fingerprint: rsaKp.fingerprint,
    encrypted_private_key: encPriv,
    security_version: 0,
    secret: false,
    created_at: 0,
    updated_at: 0
  }
}

function keyBytes(fill: number): Uint8Array {
  return new Uint8Array(32).fill(fill)
}

async function rsaWrap(bytes: Uint8Array): Promise<string> {
  return cryptfns.rsa.encryptMessage(cryptfns.uint8.toHex(bytes), rsaKp.publicKey as string)
}

describe('legacy -> curve25519 migration ceremony', () => {
  beforeEach(() => {
    posts.length = 0
    loginUser = legacyUser()
    refreshUser = null
    migrationKeys = { keys: [], link_keys: [] }
    vi.mocked(notify).mockClear()
    vi.mocked(transition.keyRotationAuditSign).mockClear()
    vi.mocked(transition.sign).mockClear()
    pk.clearRememberMe()
    setActivePinia(createPinia())
  })

  it('UNIT: re-wraps every file and link key and posts them under the right field names', async () => {
    const fileKeys = [keyBytes(11), keyBytes(12)]
    const linkKeys = [keyBytes(21), keyBytes(22), keyBytes(23)]
    const l0FileId = '11111111-1111-1111-1111-111111111111'

    migrationKeys = {
      keys: [
        { file_id: 'f0', encrypted_key: await rsaWrap(fileKeys[0]) },
        { file_id: 'f1', encrypted_key: await rsaWrap(fileKeys[1]) }
      ],
      link_keys: [
        { link_id: 'l0', encrypted_link_key: await rsaWrap(linkKeys[0]), file_id: l0FileId },
        {
          link_id: 'l1',
          encrypted_link_key: await rsaWrap(linkKeys[1]),
          file_id: '22222222-2222-2222-2222-222222222222'
        },
        {
          link_id: 'l2',
          encrypted_link_key: await rsaWrap(linkKeys[2]),
          file_id: '33333333-3333-3333-3333-333333333333'
        }
      ]
    }

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    await login.withCredentials(crypto, { email: 'legacy@test.com', password: PASSWORD })

    // The re-wraps are staged through migration/rewrap, not carried on complete.
    const rewrap = posts.find((p) => p.path === '/api/auth/migration/rewrap')
    expect(rewrap).toBeDefined()
    const staged = rewrap!.body as {
      keys: Array<Record<string, unknown>>
      link_keys: Array<Record<string, unknown>>
    }

    expect(staged.keys).toHaveLength(2)
    expect(Object.keys(staged.keys[0]).sort()).toEqual(['encrypted_key', 'file_id'])

    expect(staged.link_keys).toHaveLength(3)
    expect(Object.keys(staged.link_keys[0]).sort()).toEqual([
      'encrypted_link_key',
      'link_id',
      'signature'
    ])

    const complete = posts.find((p) => p.path === '/api/auth/migration/complete')
    expect(complete).toBeDefined()
    const body = complete!.body as {
      audit_event_signature: string
      new_identity_pubkey: string
      rewrapped_keys?: unknown
      rewrapped_link_keys?: unknown
    }

    expect(body.audit_event_signature).toBe('audit-sig')
    // complete must no longer carry the re-wraps — that was the 2 MB body bug.
    expect(body.rewrapped_keys).toBeUndefined()
    expect(body.rewrapped_link_keys).toBeUndefined()

    // Prove the re-wrap is real: a staged link key unwraps under the migrated
    // wrapping key back to the original bytes, and the old RSA key is retained.
    const newXPriv = crypto.keypair.wrappingPrivate as string
    const l0 = staged.link_keys.find((k) => k.link_id === 'l0') as {
      encrypted_link_key: string
      signature: string
    }
    const recovered = await wrapping.unwrap(l0.encrypted_link_key, newXPriv)
    expect(cryptfns.uint8.toHex(recovered)).toBe(cryptfns.uint8.toHex(linkKeys[0]))
    expect(crypto.keypair.legacyPrivate).toBe(rsaKp.input)

    // The re-signature is over the link's file_id and verifies under the new
    // identity key — the canonical the server reconstructs from its own row.
    expect(await ed25519.verify(l0FileId, l0.signature, body.new_identity_pubkey)).toBe(true)
    expect(await ed25519.verify('wrong-file-id', l0.signature, body.new_identity_pubkey)).toBe(
      false
    )
  })

  it('UNIT: signs the key-rotation audit over the server-reconstructed canonical inputs', async () => {
    migrationKeys = {
      keys: [{ file_id: 'f0', encrypted_key: await rsaWrap(keyBytes(51)) }],
      link_keys: []
    }

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    await login.withCredentials(crypto, { email: 'legacy@test.com', password: PASSWORD })

    const complete = posts.find((p) => p.path === '/api/auth/migration/complete')
    const body = complete!.body as {
      new_fingerprint: string
      transition_issued_at: number
      audit_event_signature: string
    }

    // The server re-encodes this exact canonical (user id, old fingerprint, new
    // fingerprint, rotated_at) and verifies the signature against it, so the
    // client must feed those precise inputs.
    expect(vi.mocked(transition.keyRotationAuditSign)).toHaveBeenCalledTimes(1)
    const args = vi.mocked(transition.keyRotationAuditSign).mock.calls[0][0]
    const expectedUserId = cryptfns.uint8.fromHex(legacyUser().id.replace(/-/g, ''))
    expect(Array.from(args.userId)).toEqual(Array.from(expectedUserId))
    expect(args.oldFingerprint).toBe(rsaKp.fingerprint)
    expect(args.newFingerprint).toBe(body.new_fingerprint)
    expect(args.rotatedAt).toBe(BigInt(body.transition_issued_at))
    expect(args.newIdentityPrivateKey).toBe(crypto.keypair.input)
    expect(body.audit_event_signature).toBe('audit-sig')
  })

  it('UNIT: a failure signing the key-rotation audit aborts the ceremony and posts nothing', async () => {
    migrationKeys = {
      keys: [{ file_id: 'f0', encrypted_key: await rsaWrap(keyBytes(61)) }],
      link_keys: []
    }
    vi.mocked(transition.keyRotationAuditSign).mockRejectedValueOnce(
      new Error('key_rotation_audit_sign failed')
    )

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    await login.withCredentials(crypto, { email: 'legacy@test.com', password: PASSWORD })

    expect(posts.find((p) => p.path === '/api/auth/migration/rewrap')).toBeUndefined()
    expect(posts.find((p) => p.path === '/api/auth/migration/complete')).toBeUndefined()
    expect(vi.mocked(notify)).toHaveBeenCalled()
  })

  it('UNIT: an account with zero links stages an empty link_keys array and completes', async () => {
    migrationKeys = {
      keys: [{ file_id: 'f0', encrypted_key: await rsaWrap(keyBytes(31)) }],
      link_keys: []
    }

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    await login.withCredentials(crypto, { email: 'legacy@test.com', password: PASSWORD })

    const rewrap = posts.find((p) => p.path === '/api/auth/migration/rewrap')
    expect(rewrap).toBeDefined()
    const staged = rewrap!.body as { keys: unknown[]; link_keys: unknown[] }
    expect(staged.keys).toHaveLength(1)
    expect(staged.link_keys).toEqual([])

    expect(posts.find((p) => p.path === '/api/auth/migration/complete')).toBeDefined()
  })

  it('UNIT: a single undecryptable link key aborts the ceremony, posts nothing, and warns the user', async () => {
    migrationKeys = {
      keys: [{ file_id: 'f0', encrypted_key: await rsaWrap(keyBytes(41)) }],
      link_keys: [
        {
          link_id: 'l0',
          encrypted_link_key: await rsaWrap(keyBytes(42)),
          file_id: '44444444-4444-4444-4444-444444444444'
        },
        {
          link_id: 'l1',
          encrypted_link_key: 'INVALID_LINK_KEY_BLOB',
          file_id: '55555555-5555-5555-5555-555555555555'
        }
      ]
    }

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    // The failure is swallowed into a notification so the user stays logged in;
    // the migration itself must never partially commit.
    await login.withCredentials(crypto, { email: 'legacy@test.com', password: PASSWORD })

    // The re-wrap threw before any batch was staged, so nothing was posted.
    expect(posts.find((p) => p.path === '/api/auth/migration/rewrap')).toBeUndefined()
    expect(posts.find((p) => p.path === '/api/auth/migration/complete')).toBeUndefined()
    expect(vi.mocked(notify)).toHaveBeenCalled()
  })

  it('UNIT: with remember-me, re-stores the persisted material as the migrated curve bundle', async () => {
    migrationKeys = {
      keys: [{ file_id: 'f0', encrypted_key: await rsaWrap(keyBytes(71)) }],
      link_keys: []
    }

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    const login = loginStore()
    const crypto = cryptoStore()

    await login.withCredentials(crypto, {
      email: 'legacy@test.com',
      password: PASSWORD,
      remember: true
    })

    expect(vi.mocked(notify)).not.toHaveBeenCalled()

    // The blob persisted at login held the RSA PEM keyed to the retired
    // fingerprint; after the ceremony it must carry the migrated bundle.
    const remembered = await pk.getRememberMe('dev1')
    const bundle = parseBundle(remembered as string)
    expect(bundle.identity).toBe(crypto.keypair.input)
    expect(bundle.wrapping).toBe(crypto.keypair.wrappingPrivate)
    expect(bundle.rsa).toBe(rsaKp.input)

    // Reload: fresh stores must restore the migrated identity from the
    // persisted material instead of bouncing the user to a fresh login.
    refreshUser = {
      ...legacyUser(),
      pubkey: crypto.keypair.publicKey,
      fingerprint: crypto.keypair.fingerprint,
      security_version: 1
    }
    setActivePinia(createPinia())
    const reloadedCrypto = cryptoStore()
    await loginStore().refresh(reloadedCrypto)

    expect(reloadedCrypto.keypair.keyType).toBe('curve25519')
    expect(reloadedCrypto.keypair.input).toBe(bundle.identity)
    expect(reloadedCrypto.keypair.wrappingPrivate).toBe(bundle.wrapping)
    expect(reloadedCrypto.keypair.legacyPrivate).toBe(rsaKp.input)
  })

  it('UNIT: splits a large key set into batches of at most 500 and stages each in order', async () => {
    const count = 501
    const keys: Array<{ file_id: string; encrypted_key: string }> = []
    for (let i = 0; i < count; i++) {
      keys.push({ file_id: `f${i}`, encrypted_key: await rsaWrap(keyBytes(i % 256)) })
    }
    migrationKeys = { keys, link_keys: [] }

    const { store: loginStore } = await import('../../services/auth/login')
    const { store: cryptoStore } = await import('../../services/crypto')
    await loginStore().withCredentials(cryptoStore(), {
      email: 'legacy@test.com',
      password: PASSWORD
    })

    const rewraps = posts.filter((p) => p.path === '/api/auth/migration/rewrap')
    expect(rewraps.map((p) => (p.body as { keys: unknown[] }).keys.length)).toEqual([500, 1])

    // Every key is staged exactly once, in the server's order.
    const staged = rewraps.flatMap((p) => (p.body as { keys: { file_id: string }[] }).keys)
    expect(staged.map((k) => k.file_id)).toEqual(keys.map((k) => k.file_id))
    expect(posts.find((p) => p.path === '/api/auth/migration/complete')).toBeDefined()
  })
})
