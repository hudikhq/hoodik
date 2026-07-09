import { describe, it, expect } from 'vitest'

import * as cryptfns from '../../services/cryptfns'
import * as shareCrypto from '../../services/shares/crypto'

import type { AuditEventSigInputV1, FolderMemberListV1, KeyTransitionRef, ShareEvent } from '../../types'

const FOLDER_ID = '11111111-1111-1111-1111-111111111111'
const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
const MEMBER_ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'

async function migratedSigner(): Promise<{
  oldRsa: Awaited<ReturnType<typeof cryptfns.rsa.generateKeyPair>>
  newEdPriv: string
  newEdPub: string
  transition: KeyTransitionRef
}> {
  const oldRsa = await cryptfns.rsa.generateKeyPair()
  const newEdPriv = await cryptfns.ed25519.generatePrivateKey()
  const newEdPub = await cryptfns.ed25519.publicFromPrivate(newEdPriv)

  const transition: KeyTransitionRef = {
    old_key_pem: oldRsa.publicKey as string,
    old_key_type: 'rsa',
    old_signature: '',
    new_signature: '',
    issued_at: 1_700_000_000
  }
  return { oldRsa, newEdPriv, newEdPub, transition }
}

describe('Folder roster signature — key-transition fallback', () => {
  it('UNIT: accepts a roster signed pre-migration by a since-migrated owner', async () => {
    const { oldRsa, newEdPub, transition } = await migratedSigner()

    // The owner signed the roster BEFORE migrating, so their own row embeds the
    // OLD (RSA) fingerprint and the signature is under the old RSA key.
    const oldFpHex = shareCrypto.computeFingerprint(oldRsa.publicKey as string)
    const signedInput: FolderMemberListV1 = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        {
          userId: OWNER_ID,
          pubkeyFingerprintHex: oldFpHex,
          shareRole: 'co-owner',
          isOwner: true,
          signedByUserId: OWNER_ID
        }
      ],
      membersSignedAt: BigInt(1_700_000_000)
    })
    const { signature } = await shareCrypto.signFolderMemberList(signedInput, oldRsa.input as string)

    // The server now serves the owner's CURRENT (curve) fingerprint in the row.
    const newFpHex = await cryptfns.ed25519.fingerprint(newEdPub)
    const currentInput: FolderMemberListV1 = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        {
          userId: OWNER_ID,
          pubkeyFingerprintHex: newFpHex,
          shareRole: 'co-owner',
          isOwner: true,
          signedByUserId: OWNER_ID
        }
      ],
      membersSignedAt: BigInt(1_700_000_000)
    })

    const ok = await shareCrypto.verifyFolderMemberListSignature(
      currentInput,
      signature,
      { pubkey: newEdPub, key_type: 'curve25519', key_transition: transition },
      OWNER_ID
    )
    expect(ok).toBe(true)
  })

  it('UNIT: rejects a bogus transition (old key does not match the signature)', async () => {
    const { oldRsa, newEdPub } = await migratedSigner()
    const oldFpHex = shareCrypto.computeFingerprint(oldRsa.publicKey as string)
    const signedInput: FolderMemberListV1 = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        {
          userId: OWNER_ID,
          pubkeyFingerprintHex: oldFpHex,
          shareRole: 'co-owner',
          isOwner: true,
          signedByUserId: OWNER_ID
        }
      ],
      membersSignedAt: BigInt(1_700_000_000)
    })
    const { signature } = await shareCrypto.signFolderMemberList(signedInput, oldRsa.input as string)

    // A transition pointing at an UNRELATED RSA key: the signature was never
    // made under it, so the fallback must fail rather than accept.
    const attacker = await cryptfns.rsa.generateKeyPair()
    const bogus: KeyTransitionRef = {
      old_key_pem: attacker.publicKey as string,
      old_key_type: 'rsa',
      old_signature: '',
      new_signature: '',
      issued_at: 1_700_000_000
    }

    const newFpHex = await cryptfns.ed25519.fingerprint(newEdPub)
    const currentInput: FolderMemberListV1 = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        {
          userId: OWNER_ID,
          pubkeyFingerprintHex: newFpHex,
          shareRole: 'co-owner',
          isOwner: true,
          signedByUserId: OWNER_ID
        }
      ],
      membersSignedAt: BigInt(1_700_000_000)
    })

    const ok = await shareCrypto.verifyFolderMemberListSignature(
      currentInput,
      signature,
      { pubkey: newEdPub, key_type: 'curve25519', key_transition: bogus },
      OWNER_ID
    )
    expect(ok).toBe(false)
  })

  it('UNIT: absent transition on a since-migrated signer fails (current key only)', async () => {
    const { oldRsa, newEdPub } = await migratedSigner()
    const oldFpHex = shareCrypto.computeFingerprint(oldRsa.publicKey as string)
    const signedInput: FolderMemberListV1 = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        {
          userId: OWNER_ID,
          pubkeyFingerprintHex: oldFpHex,
          shareRole: 'co-owner',
          isOwner: true,
          signedByUserId: OWNER_ID
        }
      ],
      membersSignedAt: BigInt(1_700_000_000)
    })
    const { signature } = await shareCrypto.signFolderMemberList(signedInput, oldRsa.input as string)

    const newFpHex = await cryptfns.ed25519.fingerprint(newEdPub)
    const currentInput: FolderMemberListV1 = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        {
          userId: OWNER_ID,
          pubkeyFingerprintHex: newFpHex,
          shareRole: 'co-owner',
          isOwner: true,
          signedByUserId: OWNER_ID
        }
      ],
      membersSignedAt: BigInt(1_700_000_000)
    })

    const ok = await shareCrypto.verifyFolderMemberListSignature(currentInput, signature, {
      pubkey: newEdPub,
      key_type: 'curve25519'
    })
    expect(ok).toBe(false)
  })
})

describe('Audit event signature — key-transition fallback', () => {
  function buildInput(): AuditEventSigInputV1 {
    return shareCrypto.buildAuditEventSigInput({
      senderId: OWNER_ID,
      recipientId: MEMBER_ID,
      fileId: FOLDER_ID,
      action: 'grant',
      shareRoleBefore: null,
      shareRoleAfter: 'reader',
      timestamp: BigInt(1_700_000_000)
    })
  }

  function eventRow(sig: string): ShareEvent {
    return {
      id: 'e1',
      sender_id: OWNER_ID,
      recipient_id: MEMBER_ID,
      file_id: FOLDER_ID,
      action: 'grant',
      share_role_before: null,
      share_role_after: 'reader',
      created_at: 1_700_000_000,
      prev_event_hash: null,
      this_event_hash: '',
      sender_signature: sig,
      encrypted_name: null,
      cipher: null,
      encrypted_key: null
    }
  }

  it('UNIT: accepts an event signed pre-migration (no fingerprint substitution)', async () => {
    const { oldRsa, newEdPub, transition } = await migratedSigner()
    const sig = await shareCrypto.signAuditEvent(buildInput(), oldRsa.input as string)

    const ok = await shareCrypto.verifyEventSignature(eventRow(sig), {
      pubkey: newEdPub,
      key_type: 'curve25519',
      key_transition: transition
    })
    expect(ok).toBe(true)
  })

  it('UNIT: rejects a bogus transition on an audit event', async () => {
    const { oldRsa, newEdPub } = await migratedSigner()
    const sig = await shareCrypto.signAuditEvent(buildInput(), oldRsa.input as string)

    const attacker = await cryptfns.rsa.generateKeyPair()
    const bogus: KeyTransitionRef = {
      old_key_pem: attacker.publicKey as string,
      old_key_type: 'rsa',
      old_signature: '',
      new_signature: '',
      issued_at: 1_700_000_000
    }

    const ok = await shareCrypto.verifyEventSignature(eventRow(sig), {
      pubkey: newEdPub,
      key_type: 'curve25519',
      key_transition: bogus
    })
    expect(ok).toBe(false)
  })
})
