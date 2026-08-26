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

import type { KeyPair } from 'types'

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
 * The account's search key, derived from its unlocked private key and cached
 * on the keypair for the session.
 *
 * Curve accounts derive from the wrapping key, legacy RSA accounts from their
 * RSA key — whichever one the account actually has. Migrating between the two
 * changes the key and so requires a re-index, which rides along with the
 * rewrap sweep that migration already performs.
 */
export function searchRootKey(keypair: KeyPair): string {
  if (keypair.searchKey) {
    return keypair.searchKey
  }

  const privateKey = keypair.wrappingPrivate || keypair.input

  if (!privateKey) {
    throw new Error('Cannot derive a search key without an unlocked private key')
  }

  const key = wasm.search_root_key(privateKey)

  if (!key) {
    throw new Error('Failed to derive the search key')
  }

  keypair.searchKey = key

  return key
}

/**
 * A file's search key, derived from the key the file itself is encrypted with.
 * That key is wrapped for every recipient of a share, so anyone who can open
 * the file can reproduce these tags — which is what lets a share grant skip
 * touching the index entirely.
 */
export function searchFileKey(fileKey: Uint8Array): string {
  const key = wasm.search_file_key(fileKey)

  if (!key) {
    throw new Error('Failed to derive the file search key')
  }

  return key
}

/**
 * Tag a single value: a file name for `name_hash`, or one query word.
 */
export function searchTag(key: string, value: string): string {
  const tag = wasm.search_tag(key, value)

  if (!tag) {
    throw new Error('Failed to tag the search value')
  }

  return tag
}

/**
 * Tokenize and tag text, in the `"{tag}:{weight}"` form the index accepts.
 */
export function searchTags(key: string, text: string): string[] {
  const output = wasm.search_tag_tokens(key, text) || ''

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
