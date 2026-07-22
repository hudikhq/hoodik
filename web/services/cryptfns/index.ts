import * as rsa from './rsa'
import * as aes from './aes'
import * as chacha from './chacha'
import * as cipher from './cipher'
import * as wrapping from './wrapping'
import * as ed25519 from './ed25519'
import * as opaque from './opaque'
import * as envelope from './envelope'
import * as transition from './transition'
import * as uint8 from './uint8'
import * as sha256 from './sha256'

import * as wasm from './wasm'

export {
  rsa,
  aes,
  sha256,
  uint8,
  wasm,
  chacha,
  cipher,
  wrapping,
  ed25519,
  opaque,
  envelope,
  transition
}

/**
 * Convert input string into hashed tokens
 */
export function stringToHashedTokens(s: string): string[] {
  const output = wasm.text_into_hashed_tokens(s) || ''

  return output.split(';').filter((token) => token !== '')
}

export interface LoginNonce {
  nonce: string
  timestamp: number
  canonical: string
}

/**
 * Random nonce + timestamp signed for authentication via private key. The
 * randomness keeps back-to-back logins with the same key distinguishable from
 * replays; the server rebuilds this exact canonical from the request fields
 * and refuses any nonce it has already accepted.
 */
export function createLoginNonce(fingerprint: string): LoginNonce {
  const nonce = uint8.toHex(crypto.getRandomValues(new Uint8Array(16)))
  const timestamp = Math.floor(Date.now() / 1000)

  return { nonce, timestamp, canonical: `${fingerprint}:${timestamp}:${nonce}` }
}
