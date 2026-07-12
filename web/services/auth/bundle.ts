/**
 * The private-key bundle sealed inside a migrated/v2 account's
 * `encrypted_private_key` envelope, and shown verbatim as the recovery backup
 * on the register-key screen.
 *
 * Serialised form: `v1|ed:<PEM>|x:<PEM>` for a fresh account, with an extra
 * `rsa:<PEM>` segment kept for accounts migrated from RSA (their old key still
 * decrypts pre-migration ciphertext). The `|` separator never appears inside a
 * PKCS#8 PEM, so a plain split is unambiguous.
 */
export interface KeyBundle {
  identity: string
  wrapping: string
  rsa?: string
}

export function encodeBundle(bundle: KeyBundle): string {
  const rsa = bundle.rsa ? `|rsa:${bundle.rsa}` : ''
  return `v1${rsa}|ed:${bundle.identity}|x:${bundle.wrapping}`
}

export function parseBundle(serialized: string): KeyBundle {
  let identity = ''
  let wrapping = ''
  let rsa: string | undefined
  for (const part of serialized.split('|')) {
    if (part.startsWith('ed:')) identity = part.slice(3)
    else if (part.startsWith('x:')) wrapping = part.slice(2)
    else if (part.startsWith('rsa:')) rsa = part.slice(4)
  }
  return { identity, wrapping, rsa }
}

/**
 * The recovery credential to hand a user for "log in with your key". A curve
 * account's is the full bundle (identity + wrapping, plus the retained RSA key
 * for a migrated account); a legacy RSA account's is still just its RSA private
 * key PEM. Either value is accepted verbatim by the private-key login flow.
 */
export function recoveryKeyFor(keypair: {
  keyType?: string | null
  input?: string | null
  wrappingPrivate?: string | null
  legacyPrivate?: string | null
}): string {
  if (keypair.keyType === 'curve25519' && keypair.input && keypair.wrappingPrivate) {
    return encodeBundle({
      identity: keypair.input,
      wrapping: keypair.wrappingPrivate,
      rsa: keypair.legacyPrivate ?? undefined
    })
  }
  return keypair.input ?? ''
}
