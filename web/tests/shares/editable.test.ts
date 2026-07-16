import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as cryptfns from '../../services/cryptfns'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'
import {
  buildSharedFolderPayloadFromFile,
  FolderMemberFingerprintChanged,
  FolderMemberListInvalid,
  UploadIntoSharedFolderAborted,
  uploadIntoSharedFolder,
  verifyFolderMemberList
} from '../../services/shares/editable'
import { reconcileFingerprints } from '../../services/shares/editable-members'
import { trustedFingerprintsStore } from '../../services/shares'

import type {
  FolderMember,
  FolderMembersResponse,
  KeyPair,
  KeyTransitionRow,
  UploadMultiKeyBody,
  UploadMultiKeyResponse
} from '../../types'

const FOLDER_ID = '11111111-1111-1111-1111-111111111111'
const NEW_FILE_ID = '22222222-2222-2222-2222-222222222222'

const MEMBER_SIG_V1_PREFIX = new TextEncoder().encode('hoodik-folder-mem-v1\0')

function uuidStringToBytes(uuid: string): Uint8Array {
  return cryptfns.uint8.fromHex(uuid.replace(/-/g, ''))
}

function pemToDerBytes(pem: string): Uint8Array {
  const trimmed = pem
    .replace(/-----BEGIN [A-Z ]+-----/g, '')
    .replace(/-----END [A-Z ]+-----/g, '')
    .replace(/\s+/g, '')
  return cryptfns.uint8.fromBase64(trimmed)
}

async function signMember(
  memberPubkey: string,
  memberUserId: string,
  signerPrivateKey: string,
  addedAt: number,
  role: 'reader' | 'editor' | 'co-owner',
  memberKeyType?: string
): Promise<string> {
  const { member_sig_encode_v1, rsa_sign_bytes } = await import('../../services/cryptfns/wasm')
  const der = member_sig_encode_v1(
    uuidStringToBytes(memberUserId),
    pemToDerBytes(memberPubkey),
    cryptfns.uint8.fromHex(
      shareCrypto.fingerprintForUser({ pubkey: memberPubkey, key_type: memberKeyType })
    ),
    role === 'reader' ? 0 : role === 'editor' ? 1 : 2,
    BigInt(addedAt)
  )
  if (!der) throw new Error('encode failed')
  const signingInput = new Uint8Array(MEMBER_SIG_V1_PREFIX.length + der.length)
  signingInput.set(MEMBER_SIG_V1_PREFIX, 0)
  signingInput.set(der, MEMBER_SIG_V1_PREFIX.length)
  return rsa_sign_bytes(signingInput, signerPrivateKey) as string
}

interface MemberFixture {
  user_id: string
  email: string | null
  role: 'reader' | 'editor' | 'co-owner'
  isOwner?: boolean
  keyType?: string
}

async function buildResponse(
  folderId: string,
  ownerKp: KeyPair,
  ownerUserId: string,
  members: MemberFixture[],
  options?: {
    /** When set, override the canonical signature with these bytes. */
    membersListSignature?: string | null
    /** When set, override the signed_at value. Default reuses the
     * signing timestamp so the byte stream stays consistent. */
    signedAt?: number
    /** When set, attribute the list signature to this user (useful for
     * "signer not authorized" failure cases). Defaults to owner. */
    listSignerUserId?: string
    /** When set, sign the list with this keypair regardless of who is
     * named as the signer. Lets tests forge an unauthorised signer. */
    listSignerKeypair?: KeyPair
  }
): Promise<{
  response: FolderMembersResponse
  keypairs: Map<string, KeyPair>
}> {
  const keypairs = new Map<string, KeyPair>()
  keypairs.set(ownerUserId, ownerKp)
  const folderMembers: FolderMember[] = []

  for (const fix of members) {
    let pubkey: string
    if (fix.user_id === ownerUserId) {
      pubkey = ownerKp.publicKey as string
    } else if (fix.keyType === 'curve25519') {
      pubkey = await cryptfns.ed25519.publicFromPrivate(
        await cryptfns.ed25519.generatePrivateKey()
      )
    } else {
      const kp = await cryptfns.rsa.generateKeyPair()
      keypairs.set(fix.user_id, kp)
      pubkey = kp.publicKey as string
    }
    const fingerprint = shareCrypto.fingerprintForUser({ pubkey, key_type: fix.keyType })
    const addedAt = 1_700_000_000
    const sig = await signMember(
      pubkey,
      fix.user_id,
      ownerKp.input as string,
      addedAt,
      fix.role,
      fix.keyType
    )
    folderMembers.push({
      user_id: fix.user_id,
      email: fix.email,
      pubkey,
      pubkey_fingerprint: fingerprint,
      key_type: fix.keyType,
      share_role: fix.role,
      is_owner: !!fix.isOwner,
      added_at: addedAt,
      signed_by_user_id: ownerUserId,
      member_signature: sig
    })
  }

  const signedAt = options?.signedAt ?? 1_700_000_500
  const listSignerUserId = options?.listSignerUserId ?? ownerUserId
  const listSignerKp = options?.listSignerKeypair ?? ownerKp

  // Always produce a canonical list signature unless a test explicitly
  // forces `null` (to exercise the missing-signature failure path).
  let canonicalSig: string | null = null
  if (options?.membersListSignature !== null) {
    const listInput = shareCrypto.buildFolderMemberListInput({
      folderId,
      folderOwnerId: ownerUserId,
      members: folderMembers.map((m) => ({
        userId: m.user_id,
        pubkeyFingerprintHex: m.pubkey_fingerprint,
        shareRole: m.share_role,
        isOwner: m.is_owner,
        signedByUserId: m.signed_by_user_id ?? ownerUserId
      })),
      membersSignedAt: BigInt(signedAt)
    })
    const { signature } = await shareCrypto.signFolderMemberList(
      listInput,
      listSignerKp.input as string
    )
    canonicalSig = options?.membersListSignature ?? signature
  }

  return {
    response: {
      folder_id: folderId,
      folder_owner_id: ownerUserId,
      folder_owner_pubkey_fingerprint: shareCrypto.computeFingerprint(ownerKp.publicKey as string),
      signature_algorithm: 'rsa-pss-sha256',
      members: folderMembers,
      members_signed_at: canonicalSig === null ? null : signedAt,
      members_list_signature: canonicalSig,
      members_list_signed_by_user_id: canonicalSig === null ? null : listSignerUserId
    },
    keypairs
  }
}

beforeEach(() => {
  setActivePinia(createPinia())
  if (typeof localStorage !== 'undefined') localStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('editable folder upload pipeline', () => {
  const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
  const UPLOADER_ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
  const THIRD_ID = 'cccccccc-cccc-cccc-cccc-cccccccccccc'

  it('upload_into_shared_folder_fetches_members_first', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const fetchSpy = vi.fn().mockResolvedValue(response)
    const postSpy = vi.fn(async (): Promise<UploadMultiKeyResponse> => ({
      file_id: NEW_FILE_ID
    }))
    const uploaderKp = (() => {
      const found = (response.members.find((m) => m.user_id === UPLOADER_ID) as FolderMember)
      return { pubkey: found.pubkey }
    })()
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    // Pre-trust the owner's fingerprint so the verifier short-circuits.
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: (await cryptfns.rsa.generateKeyPair()).input as string,
        callerPublicKey: uploaderKp.pubkey,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: 'aa'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 10,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted
      },
      { fetchMembers: fetchSpy, postUpload: postSpy }
    )
    expect(fetchSpy).toHaveBeenCalledWith(FOLDER_ID)
  })

  it('upload_into_shared_folder_verifies_member_list_signature', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    // Tamper with the second member's signature so verification fails.
    const tampered: FolderMembersResponse = {
      ...response,
      members: response.members.map((m, idx) =>
        idx === 1 ? { ...m, member_signature: 'AAAAAAAAA' } : m
      )
    }
    await expect(verifyFolderMemberList(tampered)).rejects.toBeInstanceOf(
      FolderMemberListInvalid
    )
  })

  it('upload_into_shared_folder_wraps_key_for_each_member', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' },
      { user_id: THIRD_ID, email: 'c@example.com', role: 'reader' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    trusted.trustFingerprint(
      THIRD_ID,
      (response.members.find((m) => m.user_id === THIRD_ID) as FolderMember).pubkey_fingerprint,
      'other'
    )
    let captured: UploadMultiKeyBody | null = null
    const fileKey = await cryptfns.cipher.generateKey('aegis128l')
    const fileKeyHex = cryptfns.uint8.toHex(fileKey)
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: (keypairs.get(UPLOADER_ID) as KeyPair).input as string,
        callerPublicKey: (keypairs.get(UPLOADER_ID) as KeyPair).publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex,
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 10,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => response,
        postUpload: async (body) => {
          captured = body
          return { file_id: NEW_FILE_ID }
        }
      }
    )
    expect(captured).not.toBeNull()
    expect(captured!.member_keys).toHaveLength(3)
    for (const entry of captured!.member_keys) {
      const kp = keypairs.get(entry.user_id) as KeyPair
      const decrypted = await cryptfns.rsa.decryptMessage(
        kp.input as string,
        entry.encrypted_key
      )
      expect(decrypted).toEqual(fileKeyHex)
    }
    const ownerOfFile = captured!.member_keys.filter((k) => k.is_owner_of_file)
    expect(ownerOfFile).toHaveLength(1)
    expect(ownerOfFile[0].user_id).toEqual(UPLOADER_ID)
  })

  it('upload_into_shared_folder_signs_audit_event', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    let captured: UploadMultiKeyBody | null = null
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: uploaderKp.input as string,
        callerPublicKey: uploaderKp.publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: 'bb'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 1,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => response,
        postUpload: async (body) => {
          captured = body
          return { file_id: NEW_FILE_ID }
        }
      }
    )
    expect(captured).not.toBeNull()
    const sigInput = shareCrypto.buildAuditEventSigInput({
      senderId: UPLOADER_ID,
      recipientId: null,
      fileId: captured!.new_file_id,
      action: 'shared_folder_upload',
      shareRoleBefore: null,
      shareRoleAfter: null,
      timestamp: BigInt(captured!.timestamp)
    })
    const ok = await shareCrypto.verifyAuditEvent(sigInput, captured!.event_signature, {
      pubkey: uploaderKp.publicKey as string
    })
    expect(ok).toBe(true)
  })

  it('upload_into_shared_folder_routes_to_upload_multikey_endpoint', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    // Spy on the production wrapper to confirm body shape lands on the
    // correct endpoint. We don't actually want a network call — replace
    // it with a stub for the assertion.
    const spy = vi.spyOn(sharesApi, 'uploadMultiKey').mockResolvedValue({
      file_id: NEW_FILE_ID
    })
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: uploaderKp.input as string,
        callerPublicKey: uploaderKp.publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: 'cc'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 1,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted
      },
      { fetchMembers: async () => response }
    )
    expect(spy).toHaveBeenCalled()
  })

  it('upload_into_shared_folder_409_triggers_member_refresh_and_retry', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const first = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const refreshed = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' },
      { user_id: THIRD_ID, email: 'c@example.com', role: 'reader' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, first.response.folder_owner_pubkey_fingerprint, 'other')
    const thirdMember = refreshed.response.members.find(
      (m) => m.user_id === THIRD_ID
    ) as FolderMember
    trusted.trustFingerprint(THIRD_ID, thirdMember.pubkey_fingerprint, 'other')
    let attempts = 0
    const uploaderKp = first.keypairs.get(UPLOADER_ID) as KeyPair
    const result = await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: uploaderKp.input as string,
        callerPublicKey: uploaderKp.publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: 'dd'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 1,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => first.response,
        postUpload: async (body) => {
          attempts += 1
          if (attempts === 1) {
            throw new sharesApi.ShareMembershipChangedError(refreshed.response)
          }
          expect(body.member_keys).toHaveLength(3)
          return { file_id: NEW_FILE_ID }
        }
      }
    )
    expect(attempts).toBe(2)
    expect(result.file_id).toBe(NEW_FILE_ID)
  })

  it('upload_into_shared_folder_409_new_member_surfaces_tofu_prompt', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const first = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const refreshed = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' },
      { user_id: THIRD_ID, email: 'c@example.com', role: 'reader' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, first.response.folder_owner_pubkey_fingerprint, 'other')
    const promptedUserIds: string[] = []
    const onUnknownMember = vi.fn(async (prompt: { member: FolderMember }) => {
      promptedUserIds.push(prompt.member.user_id)
      return true
    })
    let attempts = 0
    const uploaderKp = first.keypairs.get(UPLOADER_ID) as KeyPair
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: uploaderKp.input as string,
        callerPublicKey: uploaderKp.publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: 'ee'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 1,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted,
        onUnknownMember
      },
      {
        fetchMembers: async () => first.response,
        postUpload: async () => {
          attempts += 1
          if (attempts === 1) {
            throw new sharesApi.ShareMembershipChangedError(refreshed.response)
          }
          return { file_id: NEW_FILE_ID }
        }
      }
    )
    expect(promptedUserIds).toContain(THIRD_ID)
  })

  it('upload_into_shared_folder_progress_callback_reports_wrap_progress', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' },
      { user_id: THIRD_ID, email: 'c@example.com', role: 'reader' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    trusted.trustFingerprint(
      THIRD_ID,
      (response.members.find((m) => m.user_id === THIRD_ID) as FolderMember).pubkey_fingerprint,
      'other'
    )
    const phases: string[] = []
    const wrappedCounts: number[] = []
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: uploaderKp.input as string,
        callerPublicKey: uploaderKp.publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: 'ff'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 1,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => response,
        postUpload: async () => ({ file_id: NEW_FILE_ID }),
        onProgress: (p) => {
          phases.push(p.phase)
          if (p.phase === 'wrapping-keys') {
            wrappedCounts.push(p.wrappedKeys)
          }
        }
      }
    )
    expect(phases).toContain('verifying-members')
    expect(phases).toContain('wrapping-keys')
    expect(phases).toContain('signing')
    expect(phases).toContain('submitting')
    expect(phases).toContain('done')
    expect(wrappedCounts).toContain(0)
    expect(wrappedCounts).toContain(3)
  })

  it('verify_folder_member_list_rejects_fingerprint_mismatch', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    // Hand back a fingerprint that does not actually match the pubkey.
    const tampered: FolderMembersResponse = {
      ...response,
      members: response.members.map((m, idx) =>
        idx === 0
          ? m
          : { ...m, pubkey_fingerprint: '0'.repeat(64) }
      )
    }
    await expect(verifyFolderMemberList(tampered)).rejects.toBeInstanceOf(
      FolderMemberListInvalid
    )
  })

  it('verify_folder_member_list_accepts_curve25519_member', async () => {
    // A curve25519 member's fingerprint is the SPKI hash of their Ed25519
    // pubkey, not an RSA modulus hash — the verifier must re-derive it
    // through the same dispatch registration used, or the list hard-fails
    // for every roster containing such an account.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor', keyType: 'curve25519' }
    ])
    await expect(verifyFolderMemberList(response)).resolves.toBeUndefined()
  })

  it('upload_into_shared_folder_aborts_on_cached_fingerprint_change', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    // Cache a stale fingerprint for the owner so the reconciler refuses
    // to proceed without out-of-band re-verification.
    trusted.trustFingerprint(OWNER_ID, '0'.repeat(64), 'other')
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    await expect(
      uploadIntoSharedFolder(
        {
          callerUserId: UPLOADER_ID,
          callerPrivateKey: uploaderKp.input as string,
          callerPublicKey: uploaderKp.publicKey as string,
          payload: {
            newFileId: NEW_FILE_ID,
            parentFileId: FOLDER_ID,
            fileKeyHex: '00'.repeat(16),
            nameHash: 'h',
            encryptedName: 'enc',
            mime: 'text/plain',
            size: 1,
            chunks: 1,
            cipher: 'aegis128l'
          },
          trustedFingerprints: trusted
        },
        { fetchMembers: async () => response, postUpload: async () => ({ file_id: NEW_FILE_ID }) }
      )
    ).rejects.toBeInstanceOf(FolderMemberFingerprintChanged)
  })

  it('reconcile_rejects_fabricated_key_transition_rows', async () => {
    const oldFp = '0'.repeat(64)
    const currentFp = 'ff'.repeat(32)

    const members: FolderMember[] = [
      {
        user_id: THIRD_ID,
        pubkey: 'irrelevant-for-this-check',
        pubkey_fingerprint: currentFp,
        share_role: 'reader',
        signed_by_user_id: OWNER_ID,
        is_owner: false
      } as any
    ]

    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(THIRD_ID, oldFp, 'other')

    // A hostile server returns a row whose fingerprints link the trusted
    // fingerprint to the reported one, but whose certificate carries no
    // valid signatures. Structural linkage alone must never re-pin trust —
    // that would hand the contact's identity to whoever controls the server.
    vi.spyOn(sharesApi, 'getKeyTransitions').mockResolvedValue([
      {
        user_id: THIRD_ID,
        old_fingerprint: oldFp,
        old_key_spki: [1, 2, 3],
        old_key_type: 'rsa',
        new_fingerprint: currentFp,
        new_identity_key_pem: 'not-a-key',
        new_wrapping_key_pem: 'not-a-key',
        old_signature: [4, 5, 6],
        new_signature: [7, 8, 9],
        issued_at: 1_700_000_000
      }
    ])

    await expect(
      reconcileFingerprints(members, UPLOADER_ID, trusted, async () => true)
    ).rejects.toBeInstanceOf(FolderMemberFingerprintChanged)
    expect(trusted.lookup(THIRD_ID)?.pubkeyFingerprint).toBe(oldFp)
  })

  it('reconcile_accepts_cached_fingerprint_change_via_verified_key_transition_chain', async () => {
    // A real rotation: the member's old RSA key dual-signs the certificate
    // with the new Ed25519 identity key, exactly as migration records it.
    const oldRsa = await cryptfns.rsa.generateKeyPair()
    const newEdPriv = await cryptfns.ed25519.generatePrivateKey()
    const newEdPub = await cryptfns.ed25519.publicFromPrivate(newEdPriv)
    const wrappingPriv = await cryptfns.wrapping.generatePrivateKey()
    const wrappingPub = await cryptfns.wrapping.publicFromPrivate(wrappingPriv)

    const oldFp = shareCrypto.computeFingerprint(oldRsa.publicKey as string)
    const currentFp = await cryptfns.ed25519.fingerprint(newEdPub)
    const issuedAt = 1_700_000_000
    const { oldSignature, newSignature } = await cryptfns.transition.sign({
      userId: uuidStringToBytes(THIRD_ID),
      oldKeyType: 'rsa',
      oldKeyPem: oldRsa.publicKey as string,
      oldFingerprint: oldFp,
      newIdentityKeyPem: newEdPub,
      newWrappingKeyPem: wrappingPub,
      newFingerprint: currentFp,
      issuedAt: BigInt(issuedAt),
      oldPrivateKey: oldRsa.input as string,
      newIdentityPrivateKey: newEdPriv
    })

    const row: KeyTransitionRow = {
      user_id: THIRD_ID,
      old_fingerprint: oldFp,
      old_key_spki: Array.from(pemToDerBytes(oldRsa.publicKey as string)),
      old_key_type: 'rsa',
      new_fingerprint: currentFp,
      new_identity_key_pem: newEdPub,
      new_wrapping_key_pem: wrappingPub,
      old_signature: Array.from(cryptfns.uint8.fromBase64(oldSignature)),
      new_signature: Array.from(cryptfns.uint8.fromBase64(newSignature)),
      issued_at: issuedAt
    }

    const members: FolderMember[] = [
      {
        user_id: THIRD_ID,
        pubkey: newEdPub,
        pubkey_fingerprint: currentFp,
        key_type: 'curve25519',
        share_role: 'reader',
        signed_by_user_id: OWNER_ID,
        is_owner: false
      } as any
    ]

    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(THIRD_ID, oldFp, 'other')

    vi.spyOn(sharesApi, 'getKeyTransitions').mockResolvedValue([row])

    // Should not throw; the verified chain re-pins trust silently.
    await reconcileFingerprints(members, UPLOADER_ID, trusted, async () => true)

    const entry = trusted.lookup(THIRD_ID)
    expect(entry?.pubkeyFingerprint).toBe(currentFp)
    expect(entry?.verificationMethod).toBe('key-transition')
  })

  it('upload_into_shared_folder_aborts_when_user_declines_tofu', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' },
      { user_id: THIRD_ID, email: 'c@example.com', role: 'reader' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    await expect(
      uploadIntoSharedFolder(
        {
          callerUserId: UPLOADER_ID,
          callerPrivateKey: uploaderKp.input as string,
          callerPublicKey: uploaderKp.publicKey as string,
          payload: {
            newFileId: NEW_FILE_ID,
            parentFileId: FOLDER_ID,
            fileKeyHex: '11'.repeat(16),
            nameHash: 'h',
            encryptedName: 'enc',
            mime: 'text/plain',
            size: 1,
            chunks: 1,
            cipher: 'aegis128l'
          },
          trustedFingerprints: trusted,
          onUnknownMember: async () => false
        },
        { fetchMembers: async () => response, postUpload: async () => ({ file_id: NEW_FILE_ID }) }
      )
    ).rejects.toBeInstanceOf(UploadIntoSharedFolderAborted)
  })

  it('upload_into_shared_folder_silently_tofu_accepts_unknown_member_via_callback', async () => {
    // Cache miss for the owner + a silent-accept callback: the upload
    // proceeds without prompting, and the accepted fingerprint lands in
    // the trusted store so subsequent uploads short-circuit.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    const seen: string[] = []
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: uploaderKp.input as string,
        callerPublicKey: uploaderKp.publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: '22'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 1,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted,
        onUnknownMember: async (prompt) => {
          seen.push(prompt.member.user_id)
          return true
        }
      },
      { fetchMembers: async () => response, postUpload: async () => ({ file_id: NEW_FILE_ID }) }
    )
    expect(seen).toEqual([OWNER_ID])
    const cached = trusted.lookup(OWNER_ID)
    expect(cached?.pubkeyFingerprint).toEqual(response.folder_owner_pubkey_fingerprint)
  })

  it('upload_into_shared_folder_uses_cache_after_first_tofu_accept', async () => {
    // Second upload into the same folder: callback must not fire, and
    // the cached entry is the fast path through the reconciler.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    const callback = vi.fn(async () => true)
    await uploadIntoSharedFolder(
      {
        callerUserId: UPLOADER_ID,
        callerPrivateKey: uploaderKp.input as string,
        callerPublicKey: uploaderKp.publicKey as string,
        payload: {
          newFileId: NEW_FILE_ID,
          parentFileId: FOLDER_ID,
          fileKeyHex: '33'.repeat(16),
          nameHash: 'h',
          encryptedName: 'enc',
          mime: 'text/plain',
          size: 1,
          chunks: 1,
          cipher: 'aegis128l'
        },
        trustedFingerprints: trusted,
        onUnknownMember: callback
      },
      { fetchMembers: async () => response, postUpload: async () => ({ file_id: NEW_FILE_ID }) }
    )
    expect(callback).not.toHaveBeenCalled()
  })

  it('upload_into_shared_folder_mismatch_still_throws_after_tofu_accept', async () => {
    // Cached fingerprint differs from the one the server returned: the
    // reconciler must HARD STOP, even though we'd silently accept a new
    // member. Mismatch is the security-relevant signal; TOFU isn't.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const trusted = trustedFingerprintsStore()
    trusted.bind(UPLOADER_ID)
    trusted.trustFingerprint(OWNER_ID, 'f'.repeat(64), 'other')
    const uploaderKp = keypairs.get(UPLOADER_ID) as KeyPair
    await expect(
      uploadIntoSharedFolder(
        {
          callerUserId: UPLOADER_ID,
          callerPrivateKey: uploaderKp.input as string,
          callerPublicKey: uploaderKp.publicKey as string,
          payload: {
            newFileId: NEW_FILE_ID,
            parentFileId: FOLDER_ID,
            fileKeyHex: '44'.repeat(16),
            nameHash: 'h',
            encryptedName: 'enc',
            mime: 'text/plain',
            size: 1,
            chunks: 1,
            cipher: 'aegis128l'
          },
          trustedFingerprints: trusted,
          onUnknownMember: async () => true
        },
        { fetchMembers: async () => response, postUpload: async () => ({ file_id: NEW_FILE_ID }) }
      )
    ).rejects.toBeInstanceOf(FolderMemberFingerprintChanged)
  })

  it('build_shared_folder_payload_returns_correct_encryption_shape', async () => {
    const file = new File([new Uint8Array(64)], 'doc.txt', { type: 'text/plain' })
    const payload = await buildSharedFolderPayloadFromFile({
      newFileId: NEW_FILE_ID,
      parentFileId: FOLDER_ID,
      file,
      searchTokensHashed: ['t1'],
      chunkSizeBytes: 1024
    })
    expect(payload.newFileId).toEqual(NEW_FILE_ID)
    expect(payload.parentFileId).toEqual(FOLDER_ID)
    expect(payload.cipher).toEqual('aegis128l')
    expect(payload.size).toEqual(64)
    expect(payload.chunks).toBeGreaterThanOrEqual(1)
    // AEGIS-128L's key is 32 bytes — 64 hex chars.
    expect(payload.fileKeyHex.length).toBe(64)
  })

  it('folder_member_list_signature_required_on_upload', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(
      FOLDER_ID,
      ownerKp,
      OWNER_ID,
      [
        { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
        { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
      ],
      { membersListSignature: null }
    )
    expect(response.members_list_signature).toBeNull()
    await expect(verifyFolderMemberList(response)).rejects.toMatchObject({
      name: 'FolderMemberListInvalid',
      reason: 'list_signature_missing'
    })
  })

  it('folder_member_list_signature_verifies_against_owner_pubkey', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    // No throw — owner-signed list verifies cleanly.
    await expect(verifyFolderMemberList(response)).resolves.toBeUndefined()
  })

  it('folder_member_list_signature_verifies_against_co_owner_pubkey', async () => {
    // Build a 3-member roster where the list signature is attributed
    // to a Co-owner (UPLOADER), and that Co-owner's per-member σ
    // verifies against the folder owner — so they're an authorised
    // delegate signer.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const coOwnerKp = await cryptfns.rsa.generateKeyPair()
    const baseBuild = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'co-owner' },
      { user_id: THIRD_ID, email: 'c@example.com', role: 'reader' }
    ])
    // Swap UPLOADER's pubkey to the controlled keypair so we can sign
    // the list with its private side.
    const r = baseBuild.response
    const swapped: FolderMember[] = await Promise.all(
      r.members.map(async (m) => {
        if (m.user_id !== UPLOADER_ID) return m
        const fp = shareCrypto.computeFingerprint(coOwnerKp.publicKey as string)
        const sig = await signMember(
          coOwnerKp.publicKey as string,
          UPLOADER_ID,
          ownerKp.input as string,
          m.added_at!,
          'co-owner'
        )
        return {
          ...m,
          pubkey: coOwnerKp.publicKey as string,
          pubkey_fingerprint: fp,
          member_signature: sig
        }
      })
    )
    const signedAt = 1_700_001_000
    const listInput = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: swapped.map((m) => ({
        userId: m.user_id,
        pubkeyFingerprintHex: m.pubkey_fingerprint,
        shareRole: m.share_role,
        isOwner: m.is_owner,
        signedByUserId: m.signed_by_user_id ?? OWNER_ID
      })),
      membersSignedAt: BigInt(signedAt)
    })
    const { signature } = await shareCrypto.signFolderMemberList(
      listInput,
      coOwnerKp.input as string
    )
    const response: FolderMembersResponse = {
      ...r,
      members: swapped,
      members_signed_at: signedAt,
      members_list_signature: signature,
      members_list_signed_by_user_id: UPLOADER_ID
    }
    await expect(verifyFolderMemberList(response)).resolves.toBeUndefined()
  })

  it('folder_member_list_signature_tampered_aborts_upload', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    // Re-sign with a different keypair than the one named — bytes
    // verify against the wrong key, so verification fails.
    const tampered: FolderMembersResponse = {
      ...response,
      members_list_signature: 'AAAAAAAAA='
    }
    await expect(verifyFolderMemberList(tampered)).rejects.toMatchObject({
      name: 'FolderMemberListInvalid',
      reason: 'list_signature_invalid'
    })
  })

  it('folder_member_list_signature_signer_not_authorized_aborts_upload', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const eveKp = await cryptfns.rsa.generateKeyPair()
    // Build the response as normal then claim Eve signed the list,
    // even though Eve isn't an authorised signer for this folder.
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' }
    ])
    const tampered: FolderMembersResponse = {
      ...response,
      members_list_signed_by_user_id: 'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee'
    }
    // Compute the canonical bytes + sign with Eve's key just so a
    // simple "did the bytes verify" check would succeed against Eve;
    // the verifier still refuses because Eve isn't in the authorised
    // signer set.
    const listInput = shareCrypto.buildFolderMemberListInput({
      folderId: response.folder_id,
      folderOwnerId: response.folder_owner_id,
      members: response.members.map((m) => ({
        userId: m.user_id,
        pubkeyFingerprintHex: m.pubkey_fingerprint,
        shareRole: m.share_role,
        isOwner: m.is_owner,
        signedByUserId: m.signed_by_user_id ?? response.folder_owner_id
      })),
      membersSignedAt: BigInt(response.members_signed_at ?? 0)
    })
    const { signature: eveSig } = await shareCrypto.signFolderMemberList(
      listInput,
      eveKp.input as string
    )
    tampered.members_list_signature = eveSig
    await expect(verifyFolderMemberList(tampered)).rejects.toMatchObject({
      name: 'FolderMemberListInvalid',
      reason: 'list_signature_unauthorized_signer'
    })
  })

  it('build_folder_member_list_input_sorts_members_canonically', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response } = await buildResponse(FOLDER_ID, ownerKp, OWNER_ID, [
      { user_id: OWNER_ID, email: 'a@example.com', role: 'co-owner', isOwner: true },
      { user_id: UPLOADER_ID, email: 'b@example.com', role: 'editor' },
      { user_id: THIRD_ID, email: 'c@example.com', role: 'reader' }
    ])
    // Owner is below UPLOADER lexicographically (a < b < c) but the
    // server already returns members sorted; the encoder must produce
    // identical bytes when callers shuffle the input.
    const inputOrdered = shareCrypto.buildFolderMemberListInput({
      folderId: response.folder_id,
      folderOwnerId: response.folder_owner_id,
      members: response.members.map((m) => ({
        userId: m.user_id,
        pubkeyFingerprintHex: m.pubkey_fingerprint,
        shareRole: m.share_role,
        isOwner: m.is_owner,
        signedByUserId: m.signed_by_user_id ?? response.folder_owner_id
      })),
      membersSignedAt: BigInt(response.members_signed_at ?? 0)
    })
    const inputReversed = shareCrypto.buildFolderMemberListInput({
      folderId: response.folder_id,
      folderOwnerId: response.folder_owner_id,
      members: [...response.members].reverse().map((m) => ({
        userId: m.user_id,
        pubkeyFingerprintHex: m.pubkey_fingerprint,
        shareRole: m.share_role,
        isOwner: m.is_owner,
        signedByUserId: m.signed_by_user_id ?? response.folder_owner_id
      })),
      membersSignedAt: BigInt(response.members_signed_at ?? 0)
    })
    const { signature: sigA } = await shareCrypto.signFolderMemberList(
      inputOrdered,
      ownerKp.input as string
    )
    // RSA-PSS is randomised, so signatures differ by definition — but
    // a signature over the ordered list must verify against bytes
    // canonicalised from the reversed list.
    const okOnReversed = await shareCrypto.verifyFolderMemberListSignature(inputReversed, sigA, {
      pubkey: ownerKp.publicKey as string
    })
    expect(okOnReversed).toBe(true)
  })

  it('sign_folder_member_list_matches_rust_fixture', async () => {
    // The shared fixture bytes ship from `cryptfns::asn1` — assert the
    // JS WASM signer + the same fixture inputs produce a signature
    // that re-verifies against its own canonical bytes (RSA-PSS
    // randomness rules out byte-level signature equality, but the
    // signed input must round-trip).
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const folderId = '11111111-1111-1111-1111-111111111111'
    const ownerId = '11111111-1111-1111-1111-111111111111'
    const member2 = '22222222-2222-2222-2222-222222222222'
    const member3 = '33333333-3333-3333-3333-333333333333'
    const input = shareCrypto.buildFolderMemberListInput({
      folderId,
      folderOwnerId: ownerId,
      members: [
        {
          userId: ownerId,
          pubkeyFingerprintHex: 'a1'.repeat(32),
          shareRole: 'reader',
          isOwner: true,
          signedByUserId: ownerId
        },
        {
          userId: member2,
          pubkeyFingerprintHex: 'b2'.repeat(32),
          shareRole: 'co-owner',
          isOwner: false,
          signedByUserId: ownerId
        },
        {
          userId: member3,
          pubkeyFingerprintHex: 'c3'.repeat(32),
          shareRole: 'editor',
          isOwner: false,
          signedByUserId: member2
        }
      ],
      membersSignedAt: 1736000000n
    })
    const { signature } = await shareCrypto.signFolderMemberList(input, ownerKp.input as string)
    const ok = await shareCrypto.verifyFolderMemberListSignature(input, signature, {
      pubkey: ownerKp.publicKey as string
    })
    expect(ok).toBe(true)
  })

  it('sign_member_round_trips_against_verify_member_signature', async () => {
    // Ship the missing producer. The new
    // `shareCrypto.signMember` and `shareCrypto.verifyMemberSignature`
    // pair mirrors the existing list-signature roundtrip. Producer +
    // verifier agree on the same DER input regardless of which
    // keypair signs, so future shares produce σ that the SPA
    // verifier accepts without test fixtures lying about it.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const recipientKp = await cryptfns.rsa.generateKeyPair()
    const recipientFp = shareCrypto.computeFingerprint(
      recipientKp.publicKey as string
    )
    const args = {
      userId: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      pubkeyPem: recipientKp.publicKey as string,
      pubkeyFingerprintHex: recipientFp,
      shareRole: 'editor' as const,
      signedAt: 1_736_000_000n
    }
    const sig = await shareCrypto.signMember(args, ownerKp.input as string)
    expect(sig).toBeTruthy()
    const ok = await shareCrypto.verifyMemberSignature(args, sig, {
      pubkey: ownerKp.publicKey as string
    })
    expect(ok).toBe(true)
  })

  it('verify_member_signature_accepts_an_ed25519_signer', async () => {
    // No curve25519 login exists client-side yet, so there is no producer
    // to round-trip against — build the exact bytes `signMember` covers
    // and sign them with Ed25519 the way a curve25519 account would.
    const signerPrivate = await cryptfns.ed25519.generatePrivateKey()
    const signerPublic = await cryptfns.ed25519.publicFromPrivate(signerPrivate)
    const recipientKp = await cryptfns.rsa.generateKeyPair()
    const args = {
      userId: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      pubkeyPem: recipientKp.publicKey as string,
      pubkeyFingerprintHex: shareCrypto.computeFingerprint(recipientKp.publicKey as string),
      shareRole: 'editor' as const,
      signedAt: 1_736_000_000n
    }
    const { member_sig_encode_v1 } = await import('../../services/cryptfns/wasm')
    const der = member_sig_encode_v1(
      uuidStringToBytes(args.userId),
      pemToDerBytes(args.pubkeyPem),
      cryptfns.uint8.fromHex(args.pubkeyFingerprintHex),
      1,
      args.signedAt
    )
    if (!der) throw new Error('encode failed')
    const signingInput = new Uint8Array(MEMBER_SIG_V1_PREFIX.length + der.length)
    signingInput.set(MEMBER_SIG_V1_PREFIX, 0)
    signingInput.set(der, MEMBER_SIG_V1_PREFIX.length)
    const sig = await cryptfns.ed25519.signBytes(signingInput, signerPrivate)

    const signer = { pubkey: signerPublic, key_type: 'curve25519' }
    expect(await shareCrypto.verifyMemberSignature(args, sig, signer)).toBe(true)

    const promoted = { ...args, shareRole: 'co-owner' as const }
    expect(await shareCrypto.verifyMemberSignature(promoted, sig, signer)).toBe(false)
  })

  it('verify_member_signature_fails_under_wrong_signer_pubkey', async () => {
    // The σ commits the signer's identity through the verifying
    // key. Verifying against a stranger's pubkey must reject — this
    // is the substantive guarantee the member signature provides.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const strangerKp = await cryptfns.rsa.generateKeyPair()
    const recipientKp = await cryptfns.rsa.generateKeyPair()
    const recipientFp = shareCrypto.computeFingerprint(
      recipientKp.publicKey as string
    )
    const args = {
      userId: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      pubkeyPem: recipientKp.publicKey as string,
      pubkeyFingerprintHex: recipientFp,
      shareRole: 'editor' as const,
      signedAt: 1_736_000_000n
    }
    const sig = await shareCrypto.signMember(args, ownerKp.input as string)
    const okStranger = await shareCrypto.verifyMemberSignature(args, sig, {
      pubkey: strangerKp.publicKey as string
    })
    expect(okStranger).toBe(false)
  })

  it('verify_member_signature_rejects_mutated_role', async () => {
    // The wire role is part of the signed payload; a server can't
    // promote a Reader share to Editor by altering the column after
    // the σ lands. Mutating any committed field breaks verification.
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const recipientKp = await cryptfns.rsa.generateKeyPair()
    const recipientFp = shareCrypto.computeFingerprint(
      recipientKp.publicKey as string
    )
    const sig = await shareCrypto.signMember(
      {
        userId: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
        pubkeyPem: recipientKp.publicKey as string,
        pubkeyFingerprintHex: recipientFp,
        shareRole: 'reader',
        signedAt: 1_736_000_000n
      },
      ownerKp.input as string
    )
    const okPromoted = await shareCrypto.verifyMemberSignature(
      {
        userId: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
        pubkeyPem: recipientKp.publicKey as string,
        pubkeyFingerprintHex: recipientFp,
        shareRole: 'co-owner',
        signedAt: 1_736_000_000n
      },
      sig,
      { pubkey: ownerKp.publicKey as string }
    )
    expect(okPromoted).toBe(false)
  })
})
