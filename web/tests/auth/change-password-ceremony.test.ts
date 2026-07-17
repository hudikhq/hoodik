import { describe, it, expect, beforeEach, vi } from 'vitest'

import * as cryptfns from '../../services/cryptfns'
import * as ed25519 from '../../services/cryptfns/ed25519'
import * as wrapping from '../../services/cryptfns/wrapping'
import * as envelope from '../../services/cryptfns/envelope'
import { parseBundle } from '../../services/auth/bundle'

import type { KeyPair } from '../../types'

const posts: Array<{ path: string; body: unknown }> = []

vi.mock('../../services/api', () => ({
  default: {
    post: vi.fn(async (path: string, _query: unknown, body: unknown) => {
      posts.push({ path, body })
      if (path === '/api/auth/pake/register/start') {
        return { body: { registration_response: 'server-registration-response' } }
      }
      return { body: null }
    })
  },
  ErrorResponse: class {}
}))

// The server half of OPAQUE isn't in the client WASM, so finish is stubbed with
// a real 32-byte export key — enough to seal/open the envelope deterministically.
const EXPORT_KEY = cryptfns.uint8.toBase64(new Uint8Array(32).fill(9))
vi.mock('../../services/cryptfns/opaque', () => ({
  clientRegistrationStart: vi.fn(async () => ({ state: 'st', message: 'client-request' })),
  clientRegistrationFinish: vi.fn(async () => ({
    message: 'client-upload',
    exportKey: EXPORT_KEY
  }))
}))

describe('v2 change-password ceremony', () => {
  beforeEach(() => {
    posts.length = 0
  })

  it('UNIT: POSTs the exact PAKE field names and re-seals under the new export_key', async () => {
    const { changePasswordV2 } = await import('../../services/account')

    const edPriv = await ed25519.generatePrivateKey()
    const xPriv = await wrapping.generatePrivateKey()

    const keypair: KeyPair = {
      input: edPriv,
      publicKey: await ed25519.publicFromPrivate(edPriv),
      fingerprint: 'fp',
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: xPriv,
      wrappingPublic: await wrapping.publicFromPrivate(xPriv)
    }

    await changePasswordV2(keypair, 'a-brand-new-passphrase')

    const startPost = posts.find((p) => p.path === '/api/auth/pake/register/start')
    expect(startPost).toBeDefined()
    expect(Object.keys(startPost!.body as object)).toEqual(['registration_request'])

    const finishPost = posts.find((p) => p.path === '/api/auth/pake/register/finish')
    expect(finishPost).toBeDefined()
    const body = finishPost!.body as Record<string, unknown>
    expect(Object.keys(body).sort()).toEqual([
      'encrypted_private_key',
      'issued_at',
      'registration_upload',
      'signature',
      'token'
    ])
    expect(body.registration_upload).toBe('client-upload')
    expect(typeof body.encrypted_private_key).toBe('string')
    expect(typeof body.issued_at).toBe('number')
    expect(body.token).toBe(null)

    // The ownership proof is an Ed25519 signature over the byte-exact canonical
    // the server re-derives from its own state; pin the string so a drift on
    // either side surfaces here rather than as a silent 401.
    const canonical = `hoodik-pake-register-v1\0client-upload\0${body.issued_at}`
    expect(
      await ed25519.verify(canonical, body.signature as string, keypair.publicKey as string)
    ).toBe(true)

    // The stored envelope opens under the KEK derived from the new export_key
    // and recovers the exact in-memory keys — the account is not stranded.
    const kek = await envelope.deriveKek(cryptfns.uint8.fromBase64(EXPORT_KEY))
    const reopened = await envelope.open(kek, body.encrypted_private_key as string)
    const parsed = parseBundle(new TextDecoder().decode(reopened))
    expect(parsed.identity).toBe(edPriv)
    expect(parsed.wrapping).toBe(xPriv)
  })

  it('UNIT: forwards the TOTP token on the v2 path', async () => {
    const { changePasswordV2 } = await import('../../services/account')

    const edPriv = await ed25519.generatePrivateKey()
    const xPriv = await wrapping.generatePrivateKey()
    const keypair: KeyPair = {
      input: edPriv,
      publicKey: await ed25519.publicFromPrivate(edPriv),
      fingerprint: 'fp',
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: xPriv,
      wrappingPublic: await wrapping.publicFromPrivate(xPriv)
    }

    await changePasswordV2(keypair, 'a-brand-new-passphrase', '123456')

    const body = posts.find((p) => p.path === '/api/auth/pake/register/finish')!.body as Record<
      string,
      unknown
    >
    expect(body.token).toBe('123456')
  })

  it('UNIT: re-seals the retained RSA key when the keypair carries one', async () => {
    const { changePasswordV2 } = await import('../../services/account')

    const edPriv = await ed25519.generatePrivateKey()
    const xPriv = await wrapping.generatePrivateKey()
    const RSA_PEM = '-----BEGIN RSA PRIVATE KEY-----\nRETAINED\n-----END RSA PRIVATE KEY-----'
    const keypair: KeyPair = {
      input: edPriv,
      publicKey: await ed25519.publicFromPrivate(edPriv),
      fingerprint: 'fp',
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: xPriv,
      wrappingPublic: await wrapping.publicFromPrivate(xPriv),
      legacyPrivate: RSA_PEM
    }

    await changePasswordV2(keypair, 'a-brand-new-passphrase')

    const body = posts.find((p) => p.path === '/api/auth/pake/register/finish')!.body as Record<
      string,
      unknown
    >
    const kek = await envelope.deriveKek(cryptfns.uint8.fromBase64(EXPORT_KEY))
    const parsed = parseBundle(
      new TextDecoder().decode(await envelope.open(kek, body.encrypted_private_key as string))
    )
    expect(parsed.rsa).toBe(RSA_PEM)
    expect(parsed.identity).toBe(edPriv)
    expect(parsed.wrapping).toBe(xPriv)
  })

  it('UNIT: refuses to seal when the in-memory keys are missing', async () => {
    const { changePasswordV2 } = await import('../../services/account')
    const keypair: KeyPair = {
      input: null,
      publicKey: null,
      fingerprint: null,
      keySize: 0,
      keyType: 'curve25519'
    }
    await expect(changePasswordV2(keypair, 'pw')).rejects.toThrow()
  })
})
