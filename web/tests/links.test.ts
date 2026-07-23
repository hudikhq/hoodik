import { afterEach, describe, it, expect, vi } from 'vitest'
import * as cryptfns from '../services/cryptfns'
import * as links from '../services/links'
import type { AppLink, EncryptedAppLink } from '../types'

describe('Testing links', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('UNIT: Links can be created', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const file_key = await cryptfns.aes.generateKey()

    const link = await links.meta.createLinkFromFile(
      {
        id: '123',
        name: 'test',
        key: file_key
      },
      kp
    )

    expect(link).toBeDefined()
    expect(link.encrypted_link_key).toBeDefined()

    cryptfns.rsa.verify(link.signature, link.file_id, kp.publicKey as string)
  })

  it('UNIT: Links can be decrypted', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const file_key = await cryptfns.aes.generateKey()

    const link = await links.meta.createLinkFromFile(
      {
        id: '123',
        name: 'test',
        key: file_key
      },
      kp
    )

    const decrypted = await links.crypto.decryptOwnLink(link, kp)

    expect(decrypted).toBeDefined()
    expect(decrypted.link_key_hex).toBeDefined()
    expect(decrypted.link_key).toBeDefined()
    expect(decrypted.name).toBe('test')
    expect(decrypted.encrypted_file_key).toBeDefined()
    expect(decrypted.encrypted_thumbnail).toBeUndefined()
  })

  it('UNIT: link metadata fields never share a nonce', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const file_key = await cryptfns.aes.generateKey()

    // Name and thumbnail carry identical plaintext under the same link key.
    // With the old fixed embedded nonce their ciphertext would be byte-for-byte
    // identical (the nonce-reuse tell); a fresh nonce per field breaks that.
    const link = await links.meta.createLinkFromFile(
      { id: '123', name: 'dup', key: file_key, thumbnail: 'dup' },
      kp
    )

    expect(link.encrypted_name).not.toBe(link.encrypted_thumbnail)

    const decrypted = await links.crypto.decryptOwnLink(link, kp)
    expect(decrypted.name).toBe('dup')
    expect(decrypted.thumbnail).toBe('dup')
    expect(cryptfns.uint8.toHex(decrypted.key as Uint8Array)).toBe(cryptfns.uint8.toHex(file_key))
  })

  it('UNIT: links created before per-field nonces still decrypt', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const file_key = await cryptfns.aes.generateKey()
    const link_key = await cryptfns.aes.generateKey()

    // Reproduce the pre-fix ciphertext: fixed embedded nonce, no prefix.
    const legacy = {
      file_id: '123',
      signature: await cryptfns.rsa.sign(kp, '123'),
      encrypted_link_key: await cryptfns.rsa.encryptMessage(
        cryptfns.uint8.toHex(link_key),
        kp.publicKey as string
      ),
      encrypted_name: await cryptfns.aes.encryptToHex('legacy', link_key),
      encrypted_file_key: await cryptfns.aes.encryptToHex(cryptfns.uint8.toHex(file_key), link_key),
      encrypted_thumbnail: await cryptfns.aes.encryptToHex('thumb', link_key)
    } as unknown as EncryptedAppLink

    const decrypted = await links.crypto.decryptOwnLink(legacy, kp)

    expect(decrypted.name).toBe('legacy')
    expect(decrypted.thumbnail).toBe('thumb')
    expect(cryptfns.uint8.toHex(decrypted.key as Uint8Array)).toBe(cryptfns.uint8.toHex(file_key))
  })

  it('UNIT: curve25519 owners create and decrypt their own link', async () => {
    const edPriv = await cryptfns.ed25519.generatePrivateKey()
    const edPub = await cryptfns.ed25519.publicFromPrivate(edPriv)
    const xPriv = await cryptfns.wrapping.generatePrivateKey()
    const xPub = await cryptfns.wrapping.publicFromPrivate(xPriv)
    const file_key = await cryptfns.aes.generateKey()

    const kp = {
      input: edPriv,
      publicKey: edPub,
      fingerprint: null,
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: xPriv,
      wrappingPublic: xPub
    } as any

    const created = await links.meta.createLinkFromFile(
      { id: '123', name: 'curve', key: file_key },
      kp
    )

    expect(await cryptfns.ed25519.verify(created.file_id, created.signature, edPub)).toBe(true)

    const decrypted = await links.crypto.decryptOwnLink(created, kp)

    expect(decrypted.name).toBe('curve')
    expect(cryptfns.uint8.toHex(decrypted.key as Uint8Array)).toBe(cryptfns.uint8.toHex(file_key))
  })

  it('UNIT: isCurveKey classifies by PEM armor label, not the key body', () => {
    // Regression: a hybrid X25519+ML-KEM wrapping key's random base64 body
    // contains "RSA" ~5% of the time. Classifying by the whole PEM misroutes
    // such a curve key to the RSA path, where the link crypto throws
    // "Invalid key" — a ~5% flake on curve link create/decrypt. Only the armor
    // label is authoritative.
    const wrappingWithRsaInBody =
      '-----BEGIN HOODIK WRAPPING KEY-----\nAQIDrsaBCDwtRSAqLzAK\n-----END HOODIK WRAPPING KEY-----'
    expect(links.crypto.isCurveKey(wrappingWithRsaInBody)).toBe(true)

    const ed25519Pub = '-----BEGIN PUBLIC KEY-----\nMCowBQYDK2Vw\n-----END PUBLIC KEY-----'
    expect(links.crypto.isCurveKey(ed25519Pub)).toBe(true)

    const rsaPub = '-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEA\n-----END RSA PUBLIC KEY-----'
    const rsaPriv = '-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKC\n-----END RSA PRIVATE KEY-----'
    expect(links.crypto.isCurveKey(rsaPub)).toBe(false)
    expect(links.crypto.isCurveKey(rsaPriv)).toBe(false)

    expect(links.crypto.isCurveKey('')).toBe(false)
  })

  it('UNIT: owner signature verification dispatches by key type', async () => {
    const rsaKp = await cryptfns.rsa.generateKeyPair()
    const rsaLink = await links.meta.createLinkFromFile(
      { id: '123', name: 'rsa', key: await cryptfns.aes.generateKey() },
      rsaKp
    )

    expect(
      await links.crypto.verifyOwnerSignature({
        file_id: rsaLink.file_id,
        signature: rsaLink.signature,
        owner_pubkey: rsaKp.publicKey as string,
        owner_key_type: 'rsa'
      } as unknown as EncryptedAppLink)
    ).toBe(true)

    const edPriv = await cryptfns.ed25519.generatePrivateKey()
    const edPub = await cryptfns.ed25519.publicFromPrivate(edPriv)
    const xPriv = await cryptfns.wrapping.generatePrivateKey()
    const xPub = await cryptfns.wrapping.publicFromPrivate(xPriv)

    const curveKp = {
      input: edPriv,
      publicKey: edPub,
      wrappingPrivate: xPriv,
      wrappingPublic: xPub
    } as any

    const curveLink = await links.meta.createLinkFromFile(
      { id: '123', name: 'curve', key: await cryptfns.aes.generateKey() },
      curveKp
    )

    expect(
      await links.crypto.verifyOwnerSignature({
        file_id: curveLink.file_id,
        signature: curveLink.signature,
        owner_pubkey: edPub,
        owner_key_type: 'curve25519'
      } as unknown as EncryptedAppLink)
    ).toBe(true)
  })

  it('UNIT: multi-chunk content is fetched and decrypted per chunk', async () => {
    const cipher = cryptfns.cipher.defaultCipher()
    const file_key = await cryptfns.cipher.generateKey(cipher)

    const plaintextChunks = [
      new TextEncoder().encode('first-chunk-'),
      new TextEncoder().encode('second-chunk')
    ]
    const encryptedChunks = await Promise.all(
      plaintextChunks.map((c, i) => cryptfns.cipher.encrypt(cipher, c, file_key, i))
    )

    // file_size drives the chunk count via ceil(size / CHUNK_SIZE_BYTES); one
    // byte past a single 4 MiB chunk forces the two-request path.
    const link = {
      id: 'abc',
      file_size: 1024 * 1024 * 4 + 1,
      file_mime: 'application/octet-stream',
      file_cipher: cipher,
      name: 'multi',
      link_key: new Uint8Array(0),
      link_key_hex: '',
      key: file_key
    } as unknown as AppLink

    // The wasm pipeline issues the requests itself; serve real Responses and
    // record what crossed the wire.
    const requested: { url: string; method: string }[] = []
    vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input instanceof Request ? input.url : input)
      const method = input instanceof Request ? input.method : init?.method || 'GET'
      requested.push({ url, method })
      const match = /chunk=(\d+)/.exec(url)
      const index = match ? Number(match[1]) : 0
      return new Response(encryptedChunks[index].slice(), { status: 200 })
    })

    const data = await links.meta.downloadAndDecrypt(link)

    expect(new TextDecoder().decode(data)).toBe('first-chunk-second-chunk')
    expect(requested).toHaveLength(2)
    const urls = requested.map((r) => r.url).sort()
    expect(urls[0]).toContain('/api/links/abc?chunk=0')
    expect(urls[1]).toContain('/api/links/abc?chunk=1')
    for (const r of requested) {
      // Anonymous route, POST by design — and nothing derived from the key
      // may ever appear on the wire.
      expect(r.method).toBe('POST')
      expect(r.url).not.toContain(Buffer.from(file_key).toString('hex'))
    }
  })

  it('UNIT: content decrypt refuses to run without the file key', async () => {
    const link = { id: 'abc', file_size: 10, file_cipher: 'aegis128l' } as unknown as EncryptedAppLink
    await expect(links.meta.downloadAndDecrypt(link as AppLink)).rejects.toThrow()
  })
})
