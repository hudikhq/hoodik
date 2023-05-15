import * as rsa from './rsa'
import * as aes from './aes'
import * as chacha from './chacha'
import * as uint8 from './uint8'
import * as sha256 from './sha256'
import * as worker from './worker'

import * as wasm from './wasm'

export { rsa, aes, sha256, uint8, wasm, worker, chacha }

/**
 * Convert input string into hashed tokens
 */
export function stringToHashedTokens(s: string): string[] {
  const output = wasm.text_into_hashed_tokens(s) || ''

  return output.split(';')
}

/**
 * Create a timed nonce for authentication via private key
 */
export function createFingerprintNonce(fingerprint: string): string {
  const timestamp = parseInt(`${Date.now() / 1000}`)
  const flat = `${parseInt(`${timestamp / 60}`)}`

  return `${fingerprint}-${flat}`
}
