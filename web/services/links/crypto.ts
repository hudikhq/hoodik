import * as cryptfns from '!/cryptfns'
import type { AppLink, EncryptedAppLink, KeyPair } from 'types'

/**
 * Take the encrypted link file and decrypt it
 */
export async function decryptLinkRsa(link: EncryptedAppLink, kp: KeyPair): Promise<AppLink> {
  const link_key_hex = await cryptfns.rsa.decryptMessage(kp, link.encrypted_link_key)
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

  return {
    ...link,
    name,
    thumbnail,
    link_key,
    link_key_hex
  }
}
