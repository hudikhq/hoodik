import { FileMetadata } from '../../services/storage/metadata'
import { describe, it, expect } from 'vitest'
import * as cryptfns from '../../services/cryptfns'

describe('Testing the FileMetadata holder', () => {
  it('UNIT: Can be encrypted and decrypted back with given RSA private key', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const aesKey = await cryptfns.aes.generateKey()

    const metadata = new FileMetadata('some-file.txt', aesKey)

    const encryptedAesKey = await cryptfns.rsa.encryptMessage(
      cryptfns.aes.keyToStringJson(aesKey),
      kp.publicKey
    )

    const encrypted = await metadata.encrypt(kp.publicKey)
    const encryptedJson = JSON.parse(encrypted)

    expect(encryptedJson.key as string).not.toBe(encryptedAesKey)

    const decrypted = await FileMetadata.decrypt(encrypted, kp)

    expect(decrypted.name).toBe(metadata.name)
  })
})
