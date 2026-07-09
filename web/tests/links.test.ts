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

  it('UNIT: curve25519 owners create and decrypt their own link', async () => {
    const edPriv = await cryptfns.ed25519.generatePrivateKey()
    const edPub = await cryptfns.ed25519.publicFromPrivate(edPriv)
    const xPriv = await cryptfns.x25519.generatePrivateKey()
    const xPub = await cryptfns.x25519.publicFromPrivate(xPriv)
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
    const xPriv = await cryptfns.x25519.generatePrivateKey()
    const xPub = await cryptfns.x25519.publicFromPrivate(xPriv)

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
      plaintextChunks.map((c) => cryptfns.cipher.encrypt(cipher, c, file_key))
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

    const requested: string[] = []
    vi.stubGlobal('fetch', async (url: string) => {
      requested.push(url)
      const match = /chunk=(\d+)/.exec(url)
      const index = match ? Number(match[1]) : 0
      return {
        ok: true,
        arrayBuffer: async () => encryptedChunks[index].slice().buffer
      } as unknown as Response
    })

    const data = await links.meta.downloadAndDecrypt(link)

    expect(requested).toHaveLength(2)
    expect(requested[0]).toContain('chunk=0')
    expect(requested[1]).toContain('chunk=1')
    expect(new TextDecoder().decode(data)).toBe('first-chunk-second-chunk')
  })

  it('UNIT: content decrypt refuses to run without the file key', async () => {
    const link = { id: 'abc', file_size: 10, file_cipher: 'aegis128l' } as unknown as EncryptedAppLink
    await expect(links.meta.downloadAndDecrypt(link as AppLink)).rejects.toThrow()
  })
})
