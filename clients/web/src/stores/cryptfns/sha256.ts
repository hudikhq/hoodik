import { Sha256 } from '@openpgp/asmcrypto.js'
import { uint8 } from '.'

/**
 * Digest a string or buffer using SHA256 and return the checksum as a hex string
 */
export function digest(data: string | Uint8Array): string {
  if (typeof data === 'string') {
    data = uint8.fromUtf8(data)
  }

  return uint8.toHex(Sha256.bytes(data) as Uint8Array)
}
