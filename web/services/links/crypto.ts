import * as cryptfns from '!/cryptfns'
import type { AppLink, EncryptedAppLink, KeyPair } from 'types'

/**
 * True for a curve25519 key PEM. RSA PEMs carry "RSA" in their armor; the
 * Ed25519 and hybrid wrapping PEMs our generators emit do not, so the absence of "RSA"
 * is the discriminator — the same test `storage/meta.ts` uses on file keys.
 */
export function isCurveKey(pem: string): boolean {
  return !!pem && !pem.toUpperCase().includes('RSA')
}

/**
 * Verify the owner's signature over the link's `file_id`. curve25519 owners
 * sign with Ed25519, RSA owners with RSA-PSS, so dispatch on the owner's key
 * type — otherwise a valid curve signature is reported as invalid.
 */
export async function verifyOwnerSignature(link: EncryptedAppLink): Promise<boolean> {
  if (link.owner_key_type === 'curve25519') {
    return cryptfns.ed25519.verify(link.file_id, link.signature, link.owner_pubkey)
  }
  return cryptfns.rsa.verify(link.signature, link.file_id, link.owner_pubkey)
}

/**
 * Unwrap the link key from the owner's own wrap of it, returning it as hex.
 *
 * RSA accounts stored it as an RSA encryption of the hex string; curve25519
 * accounts stored it as a hybrid wrap blob under the owner's wrapping key.
 */
export async function unwrapOwnLinkKey(encryptedLinkKey: string, kp: KeyPair): Promise<string> {
  const wrapPriv = kp.wrappingPrivate || kp.input
  if (!wrapPriv) {
    throw new Error('Cannot unwrap link key without a private key')
  }
  if (isCurveKey(wrapPriv)) {
    const keyBytes = await cryptfns.wrapping.unwrap(encryptedLinkKey, wrapPriv)
    return cryptfns.uint8.toHex(keyBytes)
  }
  return cryptfns.rsa.decryptMessage(kp, encryptedLinkKey)
}

/**
 * Take the encrypted link and decrypt it as its owner.
 */
export async function decryptOwnLink(link: EncryptedAppLink, kp: KeyPair): Promise<AppLink> {
  const link_key_hex = await unwrapOwnLinkKey(link.encrypted_link_key, kp)
  return decryptLink(link, link_key_hex)
}

/**
 * Take the encrypted link file and decrypt it
 */
export async function decryptLink(link: EncryptedAppLink, link_key_hex: string): Promise<AppLink> {
  const link_key = cryptfns.uint8.fromHex(link_key_hex)

  const name = await cryptfns.aes.decryptFromHex(link.encrypted_name, link_key)

  const thumbnail = link.encrypted_thumbnail
    ? await cryptfns.aes.decryptFromHex(link.encrypted_thumbnail, link_key)
    : undefined

  let key: Uint8Array | undefined
  if (link.encrypted_file_key) {
    const fileKeyHex = await cryptfns.aes.decryptFromHex(link.encrypted_file_key, link_key)
    key = cryptfns.uint8.fromHex(fileKeyHex)
  }

  return {
    ...link,
    name,
    thumbnail,
    link_key,
    link_key_hex,
    key
  }
}
