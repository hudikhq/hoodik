import { Buffer } from 'buffer'
import * as forge from 'node-forge'

/**
 * Digest a string or buffer using SHA256 and return the checksum as a hex string
 */
export function digest(data: Buffer | string): string {
  if (data instanceof Buffer) {
    data = data.toString('hex')
  }

  return forge.md.sha256.create().update(data, 'raw').digest().toHex()
}
