import { init, transition_sign, key_rotation_audit_sign } from './wasm'

/**
 * Sign a transition certificate binding a user's old key to their new
 * identity and wrapping keys, with both the old and new private keys.
 * Returns the two base64 signatures the server records to prove the
 * migration was authorized by the holder of both key sets.
 */
export async function sign(args: {
  userId: Uint8Array
  oldKeyType: string
  oldKeyPem: string
  oldFingerprint: string
  newIdentityKeyPem: string
  newWrappingKeyPem: string
  newFingerprint: string
  issuedAt: bigint
  oldPrivateKey: string
  newIdentityPrivateKey: string
}): Promise<{ oldSignature: string; newSignature: string }> {
  await init()
  const json = transition_sign(
    args.userId,
    args.oldKeyType,
    args.oldKeyPem,
    args.oldFingerprint,
    args.newIdentityKeyPem,
    args.newWrappingKeyPem,
    args.newFingerprint,
    args.issuedAt,
    args.oldPrivateKey,
    args.newIdentityPrivateKey
  )

  if (!json) {
    throw new Error('transition_sign failed')
  }

  const parsed = JSON.parse(json)
  return { oldSignature: parsed.old_signature, newSignature: parsed.new_signature }
}

/**
 * Sign the key-rotation audit event with the new identity key — the first act
 * of the rotated identity, appended to the owner's audit chain so a later
 * reader sees why the signing key changed. The base64 signature is submitted as
 * `audit_event_signature`; the server re-encodes the same canonical from its
 * own record (user id, old fingerprint, new fingerprint, `rotatedAt`) and
 * verifies it before committing the migration.
 */
export async function keyRotationAuditSign(args: {
  userId: Uint8Array
  oldFingerprint: string
  newFingerprint: string
  rotatedAt: bigint
  newIdentityPrivateKey: string
}): Promise<string> {
  await init()
  const signature = key_rotation_audit_sign(
    args.userId,
    args.oldFingerprint,
    args.newFingerprint,
    args.rotatedAt,
    args.newIdentityPrivateKey
  )

  if (!signature) {
    throw new Error('key_rotation_audit_sign failed')
  }

  return signature
}
