import { init, transition_sign } from './wasm'

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
