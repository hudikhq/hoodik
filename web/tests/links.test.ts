import { describe, it, expect } from 'vitest'
import * as cryptfns from '../services/cryptfns'
import * as links from '../services/links'

describe('Testing links', () => {
  it('UNIT: Links can be created', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const file_key = await cryptfns.aes.generateKey()

    const link = await links.meta.createLinkFromFile(
      {
        id: '123',
        metadata: {
          name: 'test',
          key: file_key
        }
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
        metadata: {
          name: 'test',
          key: file_key
        }
      },
      kp
    )

    const decrypted = await links.crypto.decryptLinkRsa(link, kp)

    expect(decrypted).toBeDefined()
    expect(decrypted.link_key_hex).toBeDefined()
    expect(decrypted.link_key).toBeDefined()
    expect(decrypted.name).toBe('test')
    expect(decrypted.encrypted_file_key).toBeDefined()
    expect(decrypted.encrypted_thumbnail).toBeUndefined()
  })
})
