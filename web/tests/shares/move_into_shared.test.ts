import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as cryptfns from '../../services/cryptfns'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'
import {
  FolderMemberListInvalid,
  moveIntoSharedFolder,
  moveOutOfSharedFolder,
  moveSingleFileIntoSharedFolder
} from '../../services/shares/editable'
import { trustedFingerprintsStore } from '../../services/shares'
import {
  classifyMove,
  executeMove,
  isSharedFolder
} from '../../services/storage/moveInto'

import type {
  AppFile,
  CascadeEntry,
  FolderMember,
  FolderMembersResponse,
  KeyPair,
  MoveIntoSharedCascadeBody,
  MoveOutOfSharedBody,
  ShareRole
} from '../../types'

const FOLDER_ID = '11111111-1111-1111-1111-111111111111'
const ROOT_ID = '22222222-2222-2222-2222-222222222222'
const CHILD_A_ID = '33333333-3333-3333-3333-333333333333'
const CHILD_B_ID = '44444444-4444-4444-4444-444444444444'

const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
const CALLER_ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
const THIRD_ID = 'cccccccc-cccc-cccc-cccc-cccccccccccc'

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
  role: ShareRole
): Promise<string> {
  const { member_sig_encode_v1, rsa_sign_bytes } = await import('../../services/cryptfns/wasm')
  const der = member_sig_encode_v1(
    uuidStringToBytes(memberUserId),
    pemToDerBytes(memberPubkey),
    cryptfns.uint8.fromHex(shareCrypto.computeFingerprint(memberPubkey)),
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
  role: ShareRole
  isOwner?: boolean
}

async function buildResponse(
  ownerKp: KeyPair,
  members: MemberFixture[]
): Promise<{ response: FolderMembersResponse; keypairs: Map<string, KeyPair> }> {
  const keypairs = new Map<string, KeyPair>()
  keypairs.set(OWNER_ID, ownerKp)
  const folderMembers: FolderMember[] = []
  const addedAt = 1_700_000_000

  for (const fix of members) {
    let kp: KeyPair
    if (fix.user_id === OWNER_ID) {
      kp = ownerKp
    } else {
      kp = await cryptfns.rsa.generateKeyPair()
      keypairs.set(fix.user_id, kp)
    }
    const fingerprint = shareCrypto.computeFingerprint(kp.publicKey as string)
    const sig = await signMember(
      kp.publicKey as string,
      fix.user_id,
      ownerKp.input as string,
      addedAt,
      fix.role
    )
    folderMembers.push({
      user_id: fix.user_id,
      email: `${fix.user_id.slice(0, 4)}@example.com`,
      pubkey: kp.publicKey as string,
      pubkey_fingerprint: fingerprint,
      share_role: fix.role,
      is_owner: !!fix.isOwner,
      added_at: addedAt,
      signed_by_user_id: OWNER_ID,
      member_signature: sig
    })
  }

  const signedAt = 1_700_000_500
  const listInput = shareCrypto.buildFolderMemberListInput({
    folderId: FOLDER_ID,
    folderOwnerId: OWNER_ID,
    members: folderMembers.map((m) => ({
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
    ownerKp.input as string
  )

  return {
    response: {
      folder_id: FOLDER_ID,
      folder_owner_id: OWNER_ID,
      folder_owner_pubkey_fingerprint: shareCrypto.computeFingerprint(
        ownerKp.publicKey as string
      ),
      signature_algorithm: 'rsa-pss-sha256',
      members: folderMembers,
      members_signed_at: signedAt,
      members_list_signature: signature,
      members_list_signed_by_user_id: OWNER_ID
    },
    keypairs
  }
}

/** A file node whose `encrypted_key` is the caller's own RSA-wrap of a fresh
 *  symmetric key — exactly what the subtree walk consumes before re-wrapping. */
async function buildNode(
  id: string,
  mime: 'dir' | 'text/plain',
  callerPubkey: string
): Promise<{ node: AppFile; fileKeyHex: string }> {
  const key = await cryptfns.cipher.generateKey('aegis128l')
  const fileKeyHex = cryptfns.uint8.toHex(key)
  const encrypted_key = await cryptfns.rsa.encryptMessage(fileKeyHex, callerPubkey)
  const node: AppFile = {
    id,
    user_id: CALLER_ID,
    is_owner: true,
    name_hash: 'h',
    mime,
    chunks: 1,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key,
    encrypted_name: 'enc',
    cipher: 'aegis128l',
    name: id
  }
  return { node, fileKeyHex }
}

beforeEach(() => {
  setActivePinia(createPinia())
  if (typeof localStorage !== 'undefined') localStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('move folder cascade into shared folder', () => {
  it('wraps every node key once per member and decrypts back to the node key', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(ownerKp, [
      { user_id: OWNER_ID, role: 'co-owner', isOwner: true },
      { user_id: CALLER_ID, role: 'editor' },
      { user_id: THIRD_ID, role: 'reader' }
    ])
    const callerKp = keypairs.get(CALLER_ID) as KeyPair
    const root = await buildNode(ROOT_ID, 'dir', callerKp.publicKey as string)
    const childA = await buildNode(CHILD_A_ID, 'text/plain', callerKp.publicKey as string)
    const childB = await buildNode(CHILD_B_ID, 'text/plain', callerKp.publicKey as string)
    const nodes = [root.node, childA.node, childB.node]
    const keyById = new Map([
      [ROOT_ID, root.fileKeyHex],
      [CHILD_A_ID, childA.fileKeyHex],
      [CHILD_B_ID, childB.fileKeyHex]
    ])

    const trusted = trustedFingerprintsStore()
    trusted.bind(CALLER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')
    trusted.trustFingerprint(
      THIRD_ID,
      (response.members.find((m) => m.user_id === THIRD_ID) as FolderMember).pubkey_fingerprint,
      'other'
    )

    let captured: MoveIntoSharedCascadeBody | null = null
    await moveIntoSharedFolder(
      {
        callerUserId: CALLER_ID,
        callerPrivateKey: callerKp.input as string,
        callerWrappingPrivateKey: callerKp.input as string,
        root: root.node,
        destinationFolderId: FOLDER_ID,
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => response,
        collectSubtree: async () => nodes,
        postMove: async (body) => {
          captured = body
        }
      }
    )

    expect(captured).not.toBeNull()
    const body = captured as unknown as MoveIntoSharedCascadeBody
    expect(body.file_id).toBe(ROOT_ID)
    expect(body.destination_folder_id).toBe(FOLDER_ID)
    // entries × members matrix: one entry per node, one wrap per member.
    expect(body.entries).toHaveLength(3)
    for (const entry of body.entries as CascadeEntry[]) {
      expect(entry.member_keys).toHaveLength(3)
      const expectedKey = keyById.get(entry.file_id) as string
      for (const mk of entry.member_keys) {
        const kp = keypairs.get(mk.user_id) as KeyPair
        const decrypted = await cryptfns.rsa.decryptMessage(kp.input as string, mk.encrypted_key)
        expect(decrypted).toEqual(expectedKey)
      }
      const ownerRows = entry.member_keys.filter((k) => k.is_owner_of_file)
      expect(ownerRows).toHaveLength(1)
      expect(ownerRows[0].user_id).toBe(CALLER_ID)
    }
  })

  it('signs a shared_folder_upload audit event bound to the moved root', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(ownerKp, [
      { user_id: OWNER_ID, role: 'co-owner', isOwner: true },
      { user_id: CALLER_ID, role: 'editor' }
    ])
    const callerKp = keypairs.get(CALLER_ID) as KeyPair
    const root = await buildNode(ROOT_ID, 'dir', callerKp.publicKey as string)
    const trusted = trustedFingerprintsStore()
    trusted.bind(CALLER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')

    let captured: MoveIntoSharedCascadeBody | null = null
    const before = Math.floor(Date.now() / 1000)
    await moveIntoSharedFolder(
      {
        callerUserId: CALLER_ID,
        callerPrivateKey: callerKp.input as string,
        callerWrappingPrivateKey: callerKp.input as string,
        root: root.node,
        destinationFolderId: FOLDER_ID,
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => response,
        collectSubtree: async () => [root.node],
        postMove: async (body) => {
          captured = body
        }
      }
    )
    const body = captured as unknown as MoveIntoSharedCascadeBody
    const input = shareCrypto.buildAuditEventSigInput({
      senderId: CALLER_ID,
      recipientId: null,
      fileId: ROOT_ID,
      action: 'shared_folder_upload',
      shareRoleBefore: null,
      shareRoleAfter: null,
      timestamp: BigInt(body.timestamp)
    })
    expect(body.timestamp).toBeGreaterThanOrEqual(before)
    const ok = await shareCrypto.verifyAuditEvent(input, body.event_signature, {
      pubkey: callerKp.publicKey as string
    })
    expect(ok).toBe(true)
  })

  it('re-verifies and retries once on 409 share_membership_changed', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const first = await buildResponse(ownerKp, [
      { user_id: OWNER_ID, role: 'co-owner', isOwner: true },
      { user_id: CALLER_ID, role: 'editor' }
    ])
    const refreshed = await buildResponse(ownerKp, [
      { user_id: OWNER_ID, role: 'co-owner', isOwner: true },
      { user_id: CALLER_ID, role: 'editor' },
      { user_id: THIRD_ID, role: 'reader' }
    ])
    const callerKp = first.keypairs.get(CALLER_ID) as KeyPair
    const root = await buildNode(ROOT_ID, 'dir', callerKp.publicKey as string)
    const trusted = trustedFingerprintsStore()
    trusted.bind(CALLER_ID)
    trusted.trustFingerprint(OWNER_ID, first.response.folder_owner_pubkey_fingerprint, 'other')
    trusted.trustFingerprint(
      THIRD_ID,
      (refreshed.response.members.find((m) => m.user_id === THIRD_ID) as FolderMember)
        .pubkey_fingerprint,
      'other'
    )

    let attempts = 0
    await moveIntoSharedFolder(
      {
        callerUserId: CALLER_ID,
        callerPrivateKey: callerKp.input as string,
        callerWrappingPrivateKey: callerKp.input as string,
        root: root.node,
        destinationFolderId: FOLDER_ID,
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => first.response,
        collectSubtree: async () => [root.node],
        postMove: async (body) => {
          attempts += 1
          if (attempts === 1) {
            throw new sharesApi.ShareMembershipChangedError(refreshed.response)
          }
          expect(body.entries[0].member_keys).toHaveLength(3)
        }
      }
    )
    expect(attempts).toBe(2)
  })

  it('hard-stops on a tampered roster before any wrap', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(ownerKp, [
      { user_id: OWNER_ID, role: 'co-owner', isOwner: true },
      { user_id: CALLER_ID, role: 'editor' }
    ])
    const callerKp = keypairs.get(CALLER_ID) as KeyPair
    const root = await buildNode(ROOT_ID, 'dir', callerKp.publicKey as string)
    const tampered: FolderMembersResponse = {
      ...response,
      members: response.members.map((m, idx) =>
        idx === 1 ? { ...m, member_signature: 'AAAAAAAAA' } : m
      )
    }
    const trusted = trustedFingerprintsStore()
    trusted.bind(CALLER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')

    const post = vi.fn()
    await expect(
      moveIntoSharedFolder(
        {
          callerUserId: CALLER_ID,
          callerPrivateKey: callerKp.input as string,
          callerWrappingPrivateKey: callerKp.input as string,
          root: root.node,
          destinationFolderId: FOLDER_ID,
          trustedFingerprints: trusted
        },
        {
          fetchMembers: async () => tampered,
          collectSubtree: async () => [root.node],
          postMove: post
        }
      )
    ).rejects.toBeInstanceOf(FolderMemberListInvalid)
    expect(post).not.toHaveBeenCalled()
  })
})

describe('move single owned file into shared folder', () => {
  it('wraps the file key per member with the flat member_keys shape', async () => {
    const ownerKp = await cryptfns.rsa.generateKeyPair()
    const { response, keypairs } = await buildResponse(ownerKp, [
      { user_id: OWNER_ID, role: 'co-owner', isOwner: true },
      { user_id: CALLER_ID, role: 'editor' }
    ])
    const callerKp = keypairs.get(CALLER_ID) as KeyPair
    const file = await buildNode(ROOT_ID, 'text/plain', callerKp.publicKey as string)
    const trusted = trustedFingerprintsStore()
    trusted.bind(CALLER_ID)
    trusted.trustFingerprint(OWNER_ID, response.folder_owner_pubkey_fingerprint, 'other')

    const captured: sharesApi.MoveOutRejectedError | null = null
    let body: import('../../types').MoveIntoSharedBody | null = null
    await moveSingleFileIntoSharedFolder(
      {
        callerUserId: CALLER_ID,
        callerPrivateKey: callerKp.input as string,
        callerWrappingPrivateKey: callerKp.input as string,
        file: file.node,
        destinationFolderId: FOLDER_ID,
        trustedFingerprints: trusted
      },
      {
        fetchMembers: async () => response,
        postMove: async (b) => {
          body = b
        }
      }
    )
    void captured
    const sent = body as unknown as import('../../types').MoveIntoSharedBody
    expect(sent.file_id).toBe(ROOT_ID)
    expect(sent.member_keys).toHaveLength(2)
    for (const mk of sent.member_keys) {
      const kp = keypairs.get(mk.user_id) as KeyPair
      const decrypted = await cryptfns.rsa.decryptMessage(kp.input as string, mk.encrypted_key)
      expect(decrypted).toEqual(file.fileKeyHex)
    }
  })
})

describe('move out of shared folder', () => {
  it('sends a no-wrap body with a signed shared_folder_move_out event', async () => {
    const callerKp = await cryptfns.rsa.generateKeyPair()
    let captured: MoveOutOfSharedBody | null = null
    await moveOutOfSharedFolder(
      {
        callerUserId: CALLER_ID,
        callerPrivateKey: callerKp.input as string,
        callerWrappingPrivateKey: callerKp.input as string,
        fileId: ROOT_ID,
        destinationFolderId: null
      },
      {
        postMove: async (body) => {
          captured = body
        }
      }
    )
    const body = captured as unknown as MoveOutOfSharedBody
    expect(body.file_id).toBe(ROOT_ID)
    expect(body.destination_folder_id).toBeNull()
    expect('member_keys' in body).toBe(false)
    expect('entries' in body).toBe(false)
    const input = shareCrypto.buildAuditEventSigInput({
      senderId: CALLER_ID,
      recipientId: null,
      fileId: ROOT_ID,
      action: 'shared_folder_move_out',
      shareRoleBefore: null,
      shareRoleAfter: null,
      timestamp: BigInt(body.timestamp)
    })
    const ok = await shareCrypto.verifyAuditEvent(input, body.event_signature, {
      pubkey: callerKp.publicKey as string
    })
    expect(ok).toBe(true)
  })

  it('passes the destination folder id through when moving to a private folder', async () => {
    const callerKp = await cryptfns.rsa.generateKeyPair()
    const PRIVATE_DEST = '55555555-5555-5555-5555-555555555555'
    let captured: MoveOutOfSharedBody | null = null
    await moveOutOfSharedFolder(
      {
        callerUserId: CALLER_ID,
        callerPrivateKey: callerKp.input as string,
        callerWrappingPrivateKey: callerKp.input as string,
        fileId: ROOT_ID,
        destinationFolderId: PRIVATE_DEST
      },
      { postMove: async (body) => { captured = body } }
    )
    expect((captured as unknown as MoveOutOfSharedBody).destination_folder_id).toBe(PRIVATE_DEST)
  })
})

describe('classifyMove decision tree', () => {
  function dir(id: string, opts: Partial<AppFile> = {}): AppFile {
    return {
      id,
      user_id: CALLER_ID,
      is_owner: true,
      name_hash: 'h',
      mime: 'dir',
      chunks: 0,
      file_modified_at: 0,
      created_at: 0,
      is_new: false,
      editable: false,
      active_version: 1,
      encrypted_key: '',
      encrypted_name: 'enc',
      cipher: 'aegis128l',
      name: id,
      ...opts
    }
  }
  function file(id: string, opts: Partial<AppFile> = {}): AppFile {
    return { ...dir(id, opts), mime: 'text/plain' }
  }

  const sharedDest = dir('dest', { members_signed_at: 1_700_000_000 })
  const privateDest = dir('priv')

  it('routes a private destination to a plain move', () => {
    const d = classifyMove({
      sources: [file('f1')],
      destination: privateDest,
      sourceParent: null,
      sharingEnabled: true
    })
    expect(d.kind).toBe('plain')
  })

  it('routes an owned single file into a shared destination', () => {
    const d = classifyMove({
      sources: [file('f1')],
      destination: sharedDest,
      sourceParent: null,
      sharingEnabled: true
    })
    expect(d).toMatchObject({ kind: 'into-shared', destinationFolderId: 'dest' })
  })

  it('blocks the whole move when any source into a shared destination is not owned', () => {
    const d = classifyMove({
      sources: [file('f1'), file('f2', { is_owner: false })],
      destination: sharedDest,
      sourceParent: null,
      sharingEnabled: true
    })
    expect(d.kind).toBe('blocked')
  })

  it('routes an owned item out of a shared scope into a private destination', () => {
    const d = classifyMove({
      sources: [file('f1')],
      destination: privateDest,
      sourceParent: dir('parent', { members_signed_at: 1_700_000_000 }),
      sharingEnabled: true
    })
    expect(d).toMatchObject({ kind: 'move-out', destinationId: 'priv' })
  })

  it('blocks a non-owner trying to move an item out of a shared scope', () => {
    const d = classifyMove({
      sources: [file('f1', { is_owner: false })],
      destination: privateDest,
      sourceParent: dir('parent', { is_owner: false }),
      sharingEnabled: true
    })
    expect(d.kind).toBe('blocked')
  })

  it('treats a non-owned destination directory as shared', () => {
    expect(isSharedFolder(dir('d', { is_owner: false }))).toBe(true)
    expect(isSharedFolder(dir('d'))).toBe(false)
    expect(isSharedFolder(file('f'))).toBe(false)
    expect(isSharedFolder(null)).toBe(false)
  })

  it('falls back to a plain move when sharing is disabled', () => {
    const d = classifyMove({
      sources: [file('f1', { is_owner: false })],
      destination: sharedDest,
      sourceParent: dir('parent', { members_signed_at: 1_700_000_000 }),
      sharingEnabled: false
    })
    expect(d.kind).toBe('plain')
  })
})

describe('executeMove', () => {
  const keypair = { input: 'x', publicKey: 'y' } as unknown as KeyPair

  function file(id: string, mime: 'dir' | 'text/plain' = 'text/plain'): AppFile {
    return {
      id,
      user_id: CALLER_ID,
      is_owner: true,
      name_hash: 'h',
      mime,
      chunks: 0,
      file_modified_at: 0,
      created_at: 0,
      is_new: false,
      editable: false,
      active_version: 1,
      encrypted_key: '',
      encrypted_name: 'enc',
      cipher: 'aegis128l',
      name: id
    }
  }

  it('runs the plain delegate for a plain decision', async () => {
    const plainMove = vi.fn(async () => {})
    const result = await executeMove(
      { kind: 'plain', sources: [file('a'), file('b')], destinationId: 'dest' },
      { callerUserId: CALLER_ID, keypair, plainMove }
    )
    expect(plainMove).toHaveBeenCalledWith([file('a'), file('b')], 'dest')
    expect(result.moved).toBe(2)
  })

  it('surfaces a blocked decision without touching the wire', async () => {
    const plainMove = vi.fn(async () => {})
    const result = await executeMove(
      { kind: 'blocked', message: 'nope' },
      { callerUserId: CALLER_ID, keypair, plainMove }
    )
    expect(result.blocked).toBe('nope')
    expect(plainMove).not.toHaveBeenCalled()
  })
})
