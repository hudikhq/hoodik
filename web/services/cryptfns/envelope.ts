import { init, envelope_derive_kek, envelope_seal, envelope_open, envelope_rewrap } from './wasm'

/**
 * Derive the 32-byte key-encryption key from the OPAQUE export key.
 */
export async function deriveKek(exportKey: Uint8Array): Promise<Uint8Array> {
  await init()
  const kek = envelope_derive_kek(exportKey)

  if (!kek) {
    throw new Error('envelope_derive_kek failed')
  }

  return kek
}

/**
 * Seal a key bundle under the KEK; returns a base64 envelope.
 */
export async function seal(kek: Uint8Array, bundle: Uint8Array): Promise<string> {
  await init()
  const envelope = envelope_seal(kek, bundle)

  if (!envelope) {
    throw new Error('envelope_seal failed')
  }

  return envelope
}

/**
 * Open a base64 envelope with the KEK.
 */
export async function open(kek: Uint8Array, envelope: string): Promise<Uint8Array> {
  await init()
  const bundle = envelope_open(kek, envelope)

  if (!bundle) {
    throw new Error('envelope_open failed')
  }

  return bundle
}

/**
 * Re-wrap an envelope from the old KEK to a new one without exposing the
 * bundle; returns a fresh base64 envelope.
 */
export async function rewrap(
  oldKek: Uint8Array,
  newKek: Uint8Array,
  envelope: string
): Promise<string> {
  await init()
  const rewrapped = envelope_rewrap(oldKek, newKek, envelope)

  if (!rewrapped) {
    throw new Error('envelope_rewrap failed')
  }

  return rewrapped
}
