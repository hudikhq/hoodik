import { describe, it, expect } from 'vitest'
import * as crypto from '../services/cryptfns'

/**
 * Transition signatures are verified server-side; these tests only assert the
 * client produces two well-formed signatures for a valid set of keys.
 */
describe('Transition test', () => {
  it('UNIT: Transition: sign produces old and new signatures', async () => {
    const oldPrivateKey = await crypto.rsa.generateKeyPair()
    const identityPrivateKey = await crypto.ed25519.generatePrivateKey()
    const identityPublicKey = await crypto.ed25519.publicFromPrivate(identityPrivateKey)
    const wrappingPrivateKey = await crypto.wrapping.generatePrivateKey()
    const wrappingPublicKey = await crypto.wrapping.publicFromPrivate(wrappingPrivateKey)

    const userId = new Uint8Array(16)
    globalThis.crypto.getRandomValues(userId)

    const { oldSignature, newSignature } = await crypto.transition.sign({
      userId,
      oldKeyType: 'rsa',
      oldKeyPem: oldPrivateKey.publicKey,
      oldFingerprint: oldPrivateKey.fingerprint,
      newIdentityKeyPem: identityPublicKey,
      newWrappingKeyPem: wrappingPublicKey,
      newFingerprint: await crypto.ed25519.fingerprint(identityPublicKey),
      issuedAt: BigInt(Math.floor(Date.now() / 1000)),
      oldPrivateKey: oldPrivateKey.input,
      newIdentityPrivateKey: identityPrivateKey
    })

    expect(oldSignature.length).toBeGreaterThan(0)
    expect(newSignature.length).toBeGreaterThan(0)
  })
})
