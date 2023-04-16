import { Sha256 } from '@openpgp/asmcrypto.js'

/**
 * Digest a string or buffer using SHA256 and return the checksum as a hex string
 */
export function digest(data: string | Uint8Array): string {
  if (typeof data === 'string') {
    data = Buffer.from(data)
  }

  return Buffer.from(Sha256.bytes(data) as Uint8Array).toString('hex')
}
