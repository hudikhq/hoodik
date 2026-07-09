import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import * as cryptfns from '../../services/cryptfns'
import * as shareCrypto from '../../services/shares/crypto'
import { audit_event_sig_input_encode_v1 } from '../../services/cryptfns/wasm'
import type {
  AuditEventSigInputV1,
  ShareEntryInput
} from '../../types/shares'

const FIXTURES_DIR = resolve(__dirname, '../../../hoodik/tests/fixtures')

function fixtureBytes(name: string): Uint8Array {
  return new Uint8Array(readFileSync(resolve(FIXTURES_DIR, name)))
}

function repeat(byte: number, length: number): Uint8Array {
  return new Uint8Array(length).fill(byte)
}

function bytesToUuid(bytes: Uint8Array): string {
  const hex = cryptfns.uint8.toHex(bytes)
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
}

describe('share crypto helpers', () => {
  it('decrypt_own_file_key_roundtrips', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const fileKey = await cryptfns.aes.generateKey()
    const fileKeyHex = cryptfns.uint8.toHex(fileKey)

    const wrapped = await cryptfns.rsa.encryptMessage(fileKeyHex, kp.publicKey as string)
    const recovered = await shareCrypto.decryptOwnFileKey(wrapped, kp.input as string)

    expect(recovered).toEqual(fileKeyHex)
  })

  it('wrap_for_recipient_produces_valid_ciphertext', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const fileKey = await cryptfns.aes.generateKey()
    const fileKeyHex = cryptfns.uint8.toHex(fileKey)

    const wrapped = await shareCrypto.wrapForRecipient(fileKeyHex, {
      pubkey: kp.publicKey as string
    })
    const decrypted = await cryptfns.rsa.decryptMessage(kp, wrapped)

    expect(decrypted).toEqual(fileKeyHex)
  })

  it('wrap_for_recipient_x25519_roundtrips_raw_key_bytes', async () => {
    const identityPem = await cryptfns.ed25519.publicFromPrivate(
      await cryptfns.ed25519.generatePrivateKey()
    )
    const wrappingPrivate = await cryptfns.x25519.generatePrivateKey()
    const wrappingPublic = await cryptfns.x25519.publicFromPrivate(wrappingPrivate)
    const fileKey = await cryptfns.aes.generateKey()
    const fileKeyHex = cryptfns.uint8.toHex(fileKey)

    const blob = await shareCrypto.wrapForRecipient(fileKeyHex, {
      pubkey: identityPem,
      key_type: 'curve25519',
      wrapping_pubkey: wrappingPublic
    })
    // X25519 wraps carry the RAW key bytes (not the hex string RSA wraps
    // encrypt), so unwrapping must recover the hex-decoded key exactly.
    const recovered = await cryptfns.x25519.unwrap(blob, wrappingPrivate)

    expect(recovered).toEqual(fileKey)
  })

  it('wrap_for_recipient_curve25519_without_wrapping_pubkey_throws', async () => {
    const identityPem = await cryptfns.ed25519.publicFromPrivate(
      await cryptfns.ed25519.generatePrivateKey()
    )
    const fileKey = await cryptfns.aes.generateKey()

    await expect(
      shareCrypto.wrapForRecipient(cryptfns.uint8.toHex(fileKey), {
        pubkey: identityPem,
        key_type: 'curve25519'
      })
    ).rejects.toThrow('curve25519 recipient has no wrapping pubkey')
  })

  it('compute_entries_hash_deterministic', async () => {
    const entries: ShareEntryInput[] = [
      { file_id: bytesToUuid(repeat(0xa1, 16)), encrypted_key: cryptfns.uint8.toBase64(repeat(0xa1, 64)) },
      { file_id: bytesToUuid(repeat(0xb2, 16)), encrypted_key: cryptfns.uint8.toBase64(repeat(0xb2, 64)) }
    ]
    const reversed = [...entries].reverse()

    const hashA = await shareCrypto.computeEntriesHash(entries)
    const hashB = await shareCrypto.computeEntriesHash(reversed)

    expect(hashA).toEqual(hashB)
  })

  it('compute_entries_hash_matches_rust_fixture', async () => {
    const entries: ShareEntryInput[] = [
      {
        file_id: bytesToUuid(repeat(0xdd, 16)),
        encrypted_key: cryptfns.uint8.toBase64(repeat(0x11, 256))
      },
      {
        file_id: bytesToUuid(repeat(0x99, 16)),
        encrypted_key: cryptfns.uint8.toBase64(repeat(0x22, 256))
      }
    ]
    const hashBytes = await shareCrypto.computeEntriesHash(entries)

    // The Rust side produced `sha256(encode_entries_v1(sorted_by_file_id))`
    // for the same inputs in cryptfns/src/asn1.rs::tests::entries_fixture.
    // Computing it identically here is the cross-language equivalence proof.
    const fileIds = new Uint8Array(32)
    fileIds.set(repeat(0xdd, 16), 0)
    fileIds.set(repeat(0x99, 16), 16)
    const flat = new Uint8Array(512)
    flat.set(repeat(0x11, 256), 0)
    flat.set(repeat(0x22, 256), 256)
    const lengths = new Uint32Array([256, 256])
    const wasm = await import('../../services/cryptfns/wasm')
    const der = wasm.entries_encode_v1(fileIds, flat, lengths)
    expect(der).toBeDefined()
    const expectedHex = cryptfns.sha256.digest(der as Uint8Array)
    expect(cryptfns.uint8.toHex(hashBytes)).toEqual(expectedHex)
  })

  it('sign_share_payload_verifies_against_own_pubkey', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const senderId = bytesToUuid(repeat(0x11, 16))
    const recipientId = bytesToUuid(repeat(0x22, 16))
    const rootFileId = bytesToUuid(repeat(0x44, 16))

    const entries: ShareEntryInput[] = [
      { file_id: rootFileId, encrypted_key: cryptfns.uint8.toBase64(repeat(0x77, 64)) }
    ]
    const entriesHash = await shareCrypto.computeEntriesHash(entries)
    const recipientPubkeyFingerprintHex = cryptfns.uint8.toHex(repeat(0x33, 32))

    const payload = shareCrypto.buildSharePayload({
      senderId,
      recipientId,
      recipientPubkeyFingerprintHex,
      shareRole: 'editor',
      rootFileId,
      entriesHash,
      timestamp: 1_735_689_600n,
      nonce: repeat(0x66, 16)
    })

    const { payloadDer, signature } = await shareCrypto.signSharePayload(
      payload,
      kp.input as string
    )

    const wasm = await import('../../services/cryptfns/wasm')
    const der = cryptfns.uint8.fromBase64(payloadDer)
    const signingInput = new Uint8Array(16 + der.length)
    signingInput.set(new TextEncoder().encode('hoodik-share-v1\0'), 0)
    signingInput.set(der, 16)

    expect(wasm.rsa_verify_bytes(signingInput, signature, kp.publicKey as string)).toBe(true)
  })

  it('sign_share_payload_tampered_rejected', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const entries: ShareEntryInput[] = [
      { file_id: bytesToUuid(repeat(0x44, 16)), encrypted_key: cryptfns.uint8.toBase64(repeat(0x77, 64)) }
    ]
    const entriesHash = await shareCrypto.computeEntriesHash(entries)
    const payload = shareCrypto.buildSharePayload({
      senderId: bytesToUuid(repeat(0x11, 16)),
      recipientId: bytesToUuid(repeat(0x22, 16)),
      recipientPubkeyFingerprintHex: cryptfns.uint8.toHex(repeat(0x33, 32)),
      shareRole: 'reader',
      rootFileId: bytesToUuid(repeat(0x44, 16)),
      entriesHash,
      timestamp: 1_735_689_600n,
      nonce: repeat(0x66, 16)
    })

    const { payloadDer, signature } = await shareCrypto.signSharePayload(
      payload,
      kp.input as string
    )
    const der = cryptfns.uint8.fromBase64(payloadDer)
    der[20] ^= 0xff
    const wasm = await import('../../services/cryptfns/wasm')
    const tampered = new Uint8Array(16 + der.length)
    tampered.set(new TextEncoder().encode('hoodik-share-v1\0'), 0)
    tampered.set(der, 16)
    expect(wasm.rsa_verify_bytes(tampered, signature, kp.publicKey as string)).toBe(false)
  })

  it('sign_audit_event_verifies', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const input: AuditEventSigInputV1 = {
      senderId: repeat(0xaa, 16),
      recipientId: repeat(0xbb, 16),
      fileId: repeat(0xcc, 16),
      action: 'grant',
      shareRoleBefore: null,
      shareRoleAfter: 'editor',
      timestamp: 1_735_689_900n
    }
    const signature = await shareCrypto.signAuditEvent(input, kp.input as string)
    expect(
      await shareCrypto.verifyAuditEvent(input, signature, { pubkey: kp.publicKey as string })
    ).toBe(true)
  })

  it('sign_audit_event_role_change_includes_before_and_after_roles', async () => {
    // A role_change audit signature commits to BOTH the prior role and
    // the new role. Flipping either one (or substituting a different
    // action) breaks verification — the server reconstructs the same
    // canonical from `share_role_before` / `share_role_after` / `action`,
    // so any drift between the SPA and the row would surface as
    // `event_signature_invalid`.
    const kp = await cryptfns.rsa.generateKeyPair()
    const input = shareCrypto.buildAuditEventSigInput({
      senderId: bytesToUuid(repeat(0xaa, 16)),
      recipientId: bytesToUuid(repeat(0xbb, 16)),
      fileId: bytesToUuid(repeat(0xcc, 16)),
      action: 'role_change',
      shareRoleBefore: 'editor',
      shareRoleAfter: 'co-owner',
      timestamp: 1_735_689_900n
    })
    const signature = await shareCrypto.signAuditEvent(input, kp.input as string)
    const sender = { pubkey: kp.publicKey as string }
    expect(await shareCrypto.verifyAuditEvent(input, signature, sender)).toBe(true)

    const wrongBefore: AuditEventSigInputV1 = { ...input, shareRoleBefore: 'reader' }
    expect(await shareCrypto.verifyAuditEvent(wrongBefore, signature, sender)).toBe(false)
    const wrongAfter: AuditEventSigInputV1 = { ...input, shareRoleAfter: 'editor' }
    expect(await shareCrypto.verifyAuditEvent(wrongAfter, signature, sender)).toBe(false)
    const wrongAction: AuditEventSigInputV1 = { ...input, action: 'grant' }
    expect(await shareCrypto.verifyAuditEvent(wrongAction, signature, sender)).toBe(false)
  })

  it('sign_audit_event_against_rust_fixture', async () => {
    const der = audit_event_sig_input_encode_v1(
      repeat(0xaa, 16),
      repeat(0xbb, 16),
      repeat(0xcc, 16),
      2,
      0,
      1,
      1_735_689_900n
    )
    expect(der).toBeDefined()
    expect(new Uint8Array(der as Uint8Array)).toEqual(fixtureBytes('audit_event_sig_input_v1.der'))
  })

  it('compute_fingerprint_matches_rsa_fingerprint_public', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const wasm = await import('../../services/cryptfns/wasm')
    expect(shareCrypto.computeFingerprint(kp.publicKey as string)).toEqual(
      wasm.rsa_fingerprint_public(kp.publicKey as string)
    )
  })

  it('fingerprint_for_user_dispatches_on_key_type', async () => {
    const wasm = await import('../../services/cryptfns/wasm')

    const rsaKp = await cryptfns.rsa.generateKeyPair()
    expect(shareCrypto.fingerprintForUser({ pubkey: rsaKp.publicKey as string })).toEqual(
      wasm.rsa_fingerprint_public(rsaKp.publicKey as string)
    )

    // Registration stores `spki_fingerprint(pubkey)` for curve25519
    // accounts — the client-side re-derivation must land on the same value.
    const edPubkey = await cryptfns.ed25519.publicFromPrivate(
      await cryptfns.ed25519.generatePrivateKey()
    )
    expect(
      shareCrypto.fingerprintForUser({ pubkey: edPubkey, key_type: 'curve25519' })
    ).toEqual(wasm.spki_fingerprint(edPubkey))
  })

  it('format_fingerprint_chunks_to_quad_groups', () => {
    expect(shareCrypto.formatFingerprint('aabbccdd11223344')).toEqual('AABB-CCDD-1122-3344')
    expect(shareCrypto.formatFingerprint('abcdef'.repeat(4))).toEqual(
      'ABCD-EFAB-CDEF-ABCD-EFAB-CDEF'
    )
  })

  it('verify_audit_event_invalid_signature_returns_false', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    const input: AuditEventSigInputV1 = {
      senderId: repeat(0xaa, 16),
      recipientId: repeat(0xbb, 16),
      fileId: repeat(0xcc, 16),
      action: 'grant',
      shareRoleBefore: null,
      shareRoleAfter: 'reader',
      timestamp: 1_735_689_900n
    }
    const signature = await shareCrypto.signAuditEvent(input, kp.input as string)
    const tampered: AuditEventSigInputV1 = { ...input, timestamp: 1_735_689_901n }
    expect(
      await shareCrypto.verifyAuditEvent(tampered, signature, { pubkey: kp.publicKey as string })
    ).toBe(false)
  })

  it('entries_hash_only_uses_file_id_and_encrypted_key', async () => {
    const file_id = bytesToUuid(repeat(0xdd, 16))
    const encrypted_key = cryptfns.uint8.toBase64(repeat(0x11, 64))
    const baseEntries: ShareEntryInput[] = [{ file_id, encrypted_key }]
    const augmented: ShareEntryInput[] = [
      { file_id, encrypted_key, ...({ extra: 'ignored' } as object) } as ShareEntryInput
    ]

    const a = await shareCrypto.computeEntriesHash(baseEntries)
    const b = await shareCrypto.computeEntriesHash(augmented)
    expect(a).toEqual(b)
  })
})
