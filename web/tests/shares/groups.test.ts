import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'

import { ErrorResponse } from '../../services/api'
import * as cryptfns from '../../services/cryptfns'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'
import * as shareGroups from '../../services/shares/groups'
import { capabilitiesStore, trustedFingerprintsStore } from '../../services/shares'
import { GroupMemberFingerprintMismatch } from '../../services/shares/groups'
import ShareHubGroups from '../../src/views/shares/ShareHubGroups.vue'
import SharingPeopleAdd from '../../src/components/shares/SharingPeopleAdd.vue'

import type {
  AppShareGroup,
  AppShareGroupWithMembers,
  Capabilities,
  CreateShareEnvelope,
  FolderMembersResponse,
  GroupMemberWithKey,
  GroupsResponse
} from '../../types/shares'

const OWNER_ID = '11111111-1111-1111-1111-111111111111'
const GROUP_ID = '22222222-2222-2222-2222-222222222222'
const MEMBER_A = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
const MEMBER_B = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
const FILE_X = '33333333-3333-3333-3333-333333333333'
const FOLDER_ID = '55555555-5555-5555-5555-555555555555'
const CHILD_ID = '66666666-6666-6666-6666-666666666666'

function enabledCapabilities(overrides: Partial<Capabilities> = {}): Capabilities {
  return {
    sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
    editable_folders: true,
    share_groups: true,
    audit_log: true,
    fork: true,
    ...overrides
  }
}

function emptyGroups(): GroupsResponse {
  return { owned: [], member_of: [] }
}

function makeGroup(overrides: Partial<AppShareGroupWithMembers> = {}): AppShareGroupWithMembers {
  return {
    id: GROUP_ID,
    owner_id: OWNER_ID,
    name: 'Marketing',
    created_at: 1_700_000_000,
    members: [],
    ...overrides
  }
}

beforeEach(() => {
  setActivePinia(createPinia())
  if (typeof localStorage !== 'undefined') {
    localStorage.clear()
  }
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('group api wrappers', () => {
  it('group_create_returns_201_and_adds_to_owned_list', async () => {
    const initial = emptyGroups()
    let created: AppShareGroup | null = null
    vi.spyOn(sharesApi, 'createGroup').mockImplementation(async ({ name }) => {
      created = {
        id: GROUP_ID,
        owner_id: OWNER_ID,
        name: name as string,
        created_at: 1_700_000_000
      }
      return created
    })
    vi.spyOn(sharesApi, 'listGroups').mockImplementation(async () => {
      if (!created) return initial
      return {
        owned: [{ ...created, members: [] }],
        member_of: []
      }
    })
    const result = await shareGroups.createGroup('Marketing')
    expect(result.name).toBe('Marketing')
    const after = await sharesApi.listGroups()
    expect(after.owned.length).toBe(1)
  })

  it('group_create_duplicate_name_surfaces_409_error', async () => {
    vi.spyOn(sharesApi, 'createGroup').mockRejectedValue(new Error('409 group_name_taken'))
    await expect(shareGroups.createGroup('Marketing')).rejects.toThrow(/409|taken/)
  })

  it('group_list_separates_owned_from_member_of_and_carries_roles', async () => {
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({
      owned: [makeGroup()],
      member_of: [
        {
          id: '99999999-9999-9999-9999-999999999999',
          owner_id: MEMBER_A,
          owner_email: 'alice@example.com',
          name: "Sara's team",
          created_at: 1_700_000_500,
          added_at: 1_700_000_600,
          group_role: 'editor'
        }
      ]
    })
    const response = await sharesApi.listGroups()
    expect(response.owned.length).toBe(1)
    expect(response.member_of.length).toBe(1)
    expect(response.member_of[0].group_role).toBe('editor')
    expect(response.owned[0].owner_id).not.toBe(response.member_of[0].owner_id)
  })

  it('group_add_member_is_a_plain_roster_insert_with_nonce', async () => {
    const kpBob = await cryptfns.rsa.generateKeyPair()
    let captured: Parameters<typeof sharesApi.addGroupMember>[1] | null = null
    vi.spyOn(sharesApi, 'addGroupMember').mockImplementation(async (_groupId, body) => {
      captured = body
    })
    const recipient = {
      user_id: MEMBER_B,
      email: 'bob@example.com',
      pubkey: kpBob.publicKey as string,
      fingerprint: shareCrypto.computeFingerprint(kpBob.publicKey as string)
    }
    await shareGroups.addMember({
      groupId: GROUP_ID,
      recipient,
      groupRole: 'editor'
    })
    expect(captured).not.toBeNull()
    const body = captured!
    expect(body.user_id).toBe(MEMBER_B)
    expect(body.group_role).toBe('editor')
    expect(body.pubkey_fingerprint).toBe(recipient.fingerprint)
    // No file keys travel — adding a member never carries crypto.
    expect('existing_share_wraps' in body).toBe(false)
    expect('event_signatures' in body).toBe(false)
    expect(typeof body.timestamp).toBe('number')
    expect(cryptfns.uint8.fromBase64(body.nonce).length).toBe(16)
  })

  it('group_add_member_surfaces_server_error', async () => {
    vi.spyOn(sharesApi, 'addGroupMember').mockRejectedValue(new Error('409 conflict'))
    const kp = await cryptfns.rsa.generateKeyPair()
    await expect(
      shareGroups.addMember({
        groupId: GROUP_ID,
        recipient: {
          user_id: MEMBER_B,
          email: 'bob@example.com',
          pubkey: kp.publicKey as string,
          fingerprint: 'fp'
        },
        groupRole: 'reader'
      })
    ).rejects.toThrow(/conflict|409/)
  })

  it('group_set_member_role_targets_role_endpoint', async () => {
    const put = vi.spyOn(sharesApi, 'setGroupMemberRole').mockResolvedValue(undefined)
    await sharesApi.setGroupMemberRole(GROUP_ID, MEMBER_A, 'co-owner')
    expect(put).toHaveBeenCalledWith(GROUP_ID, MEMBER_A, 'co-owner')
  })

  it('group_remove_member_does_not_revoke_existing_shares', async () => {
    const removeSpy = vi.spyOn(sharesApi, 'removeGroupMember').mockResolvedValue(undefined)
    const revokeSpy = vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
    await sharesApi.removeGroupMember(GROUP_ID, MEMBER_A)
    expect(removeSpy).toHaveBeenCalledWith(GROUP_ID, MEMBER_A)
    expect(revokeSpy).not.toHaveBeenCalled()
  })
})

describe('share-to-group fan-out', () => {
  // The fingerprint defaults to the one derived from `pubkey` so fixtures are
  // self-consistent by construction; the adversarial case passes an explicit
  // mismatching fingerprint to model a lying server.
  function makeMemberWithKey(
    userId: string,
    pubkey: string,
    fingerprint: string = shareCrypto.computeFingerprint(pubkey)
  ): GroupMemberWithKey {
    return { user_id: userId, email: `${userId}@example.com`, pubkey, fingerprint, group_role: 'reader' }
  }

  // The fan-out reads the root file's existing recipients to pick the right
  // audit action per member. Default to none so each test models a fresh share
  // unless it explicitly seeds a prior role.
  beforeEach(() => {
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([])
  })

  it('share_to_group_fans_out_one_share_per_member', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    const kpCarol = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const fileKeyHex = cryptfns.uint8.toHex(fileKey)
    const ownerWrap = await cryptfns.rsa.encryptMessage(fileKeyHex, kpOwner.publicKey as string)

    const members = [
      makeMemberWithKey(MEMBER_A, kpBob.publicKey as string),
      makeMemberWithKey(MEMBER_B, kpCarol.publicKey as string)
    ]
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue(members)
    const envelopes: CreateShareEnvelope[] = []
    vi.spyOn(sharesApi, 'createShare').mockImplementation(async (envelope) => {
      envelopes.push(envelope)
      return { shares: [] }
    })

    await shareGroups.shareToGroup({
      groupId: GROUP_ID,
      root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never,
      subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never],
      shareRole: 'editor',
      senderId: OWNER_ID,
      privateKey: kpOwner.input as string,
      wrappingPrivateKey: kpOwner.input as string,
      trusted: trustedFingerprintsStore()
    })

    // One single-share POST per member — the legacy single group-shares
    // endpoint is gone; this is a client-side fan-out.
    expect(envelopes.length).toBe(2)

    // Every member's wrap decrypts to the same plaintext file key under THEIR
    // private key — proves a per-member wrap, not a copy.
    const decryptedBob = await cryptfns.rsa.decryptMessage(kpBob, envelopes[0].entries[0].encrypted_key)
    const decryptedCarol = await cryptfns.rsa.decryptMessage(kpCarol, envelopes[1].entries[0].encrypted_key)
    expect(decryptedBob).toEqual(fileKeyHex)
    expect(decryptedCarol).toEqual(fileKeyHex)

    // Each envelope carries a per-recipient member signature over the shared
    // file role, verifiable against the owner's pubkey.
    for (const env of envelopes) {
      expect(env.member_signature).toBeTruthy()
      expect(env.event_signature).toBeTruthy()
    }
  })

  it('share_to_group_excludes_the_caller_and_the_file_owner', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpEditor = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const editorWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpEditor.publicKey as string
    )

    // The roster lists the owner (group owner), the editor (caller), and Bob.
    // The caller can't share to themselves, and the file owner already owns
    // the file — only Bob should receive a fresh share.
    const roster: GroupMemberWithKey[] = [
      { user_id: OWNER_ID, email: 'owner@example.com', pubkey: kpOwner.publicKey as string, fingerprint: shareCrypto.computeFingerprint(kpOwner.publicKey as string), group_role: 'owner' },
      makeMemberWithKey(MEMBER_A, kpEditor.publicKey as string),
      makeMemberWithKey(MEMBER_B, kpBob.publicKey as string)
    ]
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue(roster)
    const envelopes: CreateShareEnvelope[] = []
    vi.spyOn(sharesApi, 'createShare').mockImplementation(async (envelope) => {
      envelopes.push(envelope)
      return { shares: [] }
    })

    await shareGroups.shareToGroup({
      groupId: GROUP_ID,
      // A row opened from "Shared with me" carries the owner's id in `user_id`
      // (the SPA maps `owner_id` onto it), so the owner is filtered out here.
      root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: editorWrap, is_owner: false } as never,
      subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: editorWrap, is_owner: false } as never],
      shareRole: 'reader',
      senderId: MEMBER_A,
      privateKey: kpEditor.input as string,
      wrappingPrivateKey: kpEditor.input as string,
      trusted: trustedFingerprintsStore()
    })

    // Exactly one envelope, wrapping for Bob only.
    expect(envelopes.length).toBe(1)
    const decrypted = await cryptfns.rsa.decryptMessage(kpBob, envelopes[0].entries[0].encrypted_key)
    expect(decrypted).toEqual(cryptfns.uint8.toHex(fileKey))
  })

  it('share_to_group_skips_a_recipient_the_server_says_already_owns_the_file', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpEditor = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const editorWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpEditor.publicKey as string
    )

    // A row opened from *inside* a shared folder (loaded via the storage
    // listing) carries the CALLER's id in `user_id`, so the roster filter
    // can't drop the owner — the owner stays a recipient and the server
    // rejects that one grant. The fan-out must skip it and still reach Bob,
    // not abort the whole group.
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      { user_id: OWNER_ID, email: 'owner@example.com', pubkey: kpOwner.publicKey as string, fingerprint: shareCrypto.computeFingerprint(kpOwner.publicKey as string), group_role: 'owner' },
      makeMemberWithKey(MEMBER_B, kpBob.publicKey as string)
    ])
    const shared: string[] = []
    vi.spyOn(sharesApi, 'createShare').mockImplementation(async (envelope) => {
      // The owner's envelope is the one whose wrapped key decrypts under the
      // owner's private key; the server rejects exactly that grant.
      const forOwner = await cryptfns.rsa
        .decryptMessage(kpOwner, envelope.entries[0].encrypted_key)
        .then(() => true)
        .catch(() => false)
      if (forOwner) {
        throw new ErrorResponse({
          request: { method: 'post', url: '/api/shares' },
          status: 400,
          headers: new Headers(),
          rawBody: undefined,
          body: { status: 400, message: 'cannot_share_owner_row' }
        } as never)
      }
      shared.push(envelope.entries[0].file_id)
      return { shares: [] }
    })

    await expect(
      shareGroups.shareToGroup({
        groupId: GROUP_ID,
        root: { id: FILE_X, user_id: MEMBER_A, mime: 'application/pdf', encrypted_key: editorWrap, is_owner: false } as never,
        subtree: [{ id: FILE_X, user_id: MEMBER_A, mime: 'application/pdf', encrypted_key: editorWrap, is_owner: false } as never],
        shareRole: 'reader',
        senderId: MEMBER_A,
        privateKey: kpEditor.input as string,
        wrappingPrivateKey: kpEditor.input as string,
        trusted: trustedFingerprintsStore()
      })
    ).resolves.toBeUndefined()

    // The owner's grant was rejected and skipped; Bob's still went through.
    expect(shared).toEqual([FILE_X])
  })

  it('share_to_group_signs_role_change_for_a_member_who_already_holds_the_file', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    const kpCarol = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    const fpBob = shareCrypto.computeFingerprint(kpBob.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      makeMemberWithKey(MEMBER_A, kpBob.publicKey as string, fpBob),
      makeMemberWithKey(MEMBER_B, kpCarol.publicKey as string)
    ])

    // Bob already holds the root file as a reader; the group share moves him to
    // editor. The server reconstructs `role_change` (before=reader) from its own
    // state and verifies against that, so the envelope must sign the transition
    // — not a `grant`. Carol is fresh and must still get a `grant`.
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([
      {
        file_id: FILE_X,
        recipient_id: MEMBER_A,
        recipient_email: 'bob@example.com',
        recipient_pubkey_fingerprint: fpBob,
        share_role: 'reader',
        created_at: 1,
        shared_at: 1,
        shared_by_user_id: OWNER_ID,
        shared_by_email: 'owner@example.com'
      }
    ])

    const envelopes: CreateShareEnvelope[] = []
    vi.spyOn(sharesApi, 'createShare').mockImplementation(async (envelope) => {
      envelopes.push(envelope)
      return { shares: [] }
    })

    await shareGroups.shareToGroup({
      groupId: GROUP_ID,
      root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never,
      subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never],
      shareRole: 'editor',
      senderId: OWNER_ID,
      privateKey: kpOwner.input as string,
      wrappingPrivateKey: kpOwner.input as string,
      trusted: trustedFingerprintsStore()
    })

    expect(envelopes.length).toBe(2)

    // Bob's envelope must verify as a `role_change` from reader→editor against
    // the owner's pubkey; the `grant` canonical for the same move must fail.
    const bobEnvelope = (
      await Promise.all(
        envelopes.map(async (env) => ({
          env,
          forBob: await cryptfns.rsa
            .decryptMessage(kpBob, env.entries[0].encrypted_key)
            .then(() => true)
            .catch(() => false)
        }))
      )
    ).find((e) => e.forBob)!.env

    const roleChangeInput = shareCrypto.buildAuditEventSigInput({
      senderId: OWNER_ID,
      recipientId: MEMBER_A,
      fileId: FILE_X,
      action: 'role_change',
      shareRoleBefore: 'reader',
      shareRoleAfter: 'editor',
      timestamp: BigInt(bobEnvelope.member_signed_at as number)
    })
    expect(
      await shareCrypto.verifyAuditEvent(roleChangeInput, bobEnvelope.event_signature, {
        pubkey: kpOwner.publicKey as string
      })
    ).toBe(true)

    const grantInput = shareCrypto.buildAuditEventSigInput({
      senderId: OWNER_ID,
      recipientId: MEMBER_A,
      fileId: FILE_X,
      action: 'grant',
      shareRoleBefore: null,
      shareRoleAfter: 'editor',
      timestamp: BigInt(bobEnvelope.member_signed_at as number)
    })
    expect(
      await shareCrypto.verifyAuditEvent(grantInput, bobEnvelope.event_signature, {
        pubkey: kpOwner.publicKey as string
      })
    ).toBe(false)
  })

  it('share_to_group_skips_a_member_who_already_holds_the_file_at_the_same_role', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    const kpCarol = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    const fpBob = shareCrypto.computeFingerprint(kpBob.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      makeMemberWithKey(MEMBER_A, kpBob.publicKey as string, fpBob),
      makeMemberWithKey(MEMBER_B, kpCarol.publicKey as string)
    ])
    // Bob already holds the file at the role being shared; the server would
    // no-op his grant, so the fan-out skips him and only Carol is POSTed.
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([
      {
        file_id: FILE_X,
        recipient_id: MEMBER_A,
        recipient_email: 'bob@example.com',
        recipient_pubkey_fingerprint: fpBob,
        share_role: 'editor',
        created_at: 1,
        shared_at: 1,
        shared_by_user_id: OWNER_ID,
        shared_by_email: 'owner@example.com'
      }
    ])

    const recipients: string[] = []
    vi.spyOn(sharesApi, 'createShare').mockImplementation(async (envelope) => {
      const forCarol = await cryptfns.rsa
        .decryptMessage(kpCarol, envelope.entries[0].encrypted_key)
        .then(() => true)
        .catch(() => false)
      recipients.push(forCarol ? MEMBER_B : MEMBER_A)
      return { shares: [] }
    })

    let lastProgress: [number, number] = [0, 0]
    await shareGroups.shareToGroup({
      groupId: GROUP_ID,
      root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never,
      subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never],
      shareRole: 'editor',
      senderId: OWNER_ID,
      privateKey: kpOwner.input as string,
      wrappingPrivateKey: kpOwner.input as string,
      trusted: trustedFingerprintsStore(),
      onProgress: (done, total) => {
        lastProgress = [done, total]
      }
    })

    expect(recipients).toEqual([MEMBER_B])
    // Progress still counts the skipped member so the bar reaches 100%.
    expect(lastProgress).toEqual([2, 2])
  })

  it('share_to_group_signs_grant_not_role_change_for_a_same_role_folder_member', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    const kpCarol = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    const fpBob = shareCrypto.computeFingerprint(kpBob.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      makeMemberWithKey(MEMBER_A, kpBob.publicKey as string, fpBob),
      makeMemberWithKey(MEMBER_B, kpCarol.publicKey as string)
    ])

    // Bob already holds the folder root at the role being shared. For a single
    // file that's a pure no-op (skipped above), but a folder still has to
    // back-fill the descendants Bob doesn't hold — so he is POSTed. Because the
    // root role is not moving, the server reconstructs a `grant` (before=null),
    // not a `role_change`; signing role_change here would 400 the whole fan-out.
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([
      {
        file_id: FOLDER_ID,
        recipient_id: MEMBER_A,
        recipient_email: 'bob@example.com',
        recipient_pubkey_fingerprint: fpBob,
        share_role: 'reader',
        created_at: 1,
        shared_at: 1,
        shared_by_user_id: OWNER_ID,
        shared_by_email: 'owner@example.com'
      }
    ])

    const envelopes: CreateShareEnvelope[] = []
    vi.spyOn(sharesApi, 'createShare').mockImplementation(async (envelope) => {
      envelopes.push(envelope)
      return { shares: [] }
    })

    await shareGroups.shareToGroup({
      groupId: GROUP_ID,
      root: { id: FOLDER_ID, user_id: OWNER_ID, mime: 'dir', encrypted_key: ownerWrap, is_owner: true } as never,
      subtree: [
        { id: FOLDER_ID, user_id: OWNER_ID, mime: 'dir', encrypted_key: ownerWrap, is_owner: true } as never,
        { id: CHILD_ID, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never
      ],
      shareRole: 'reader',
      senderId: OWNER_ID,
      privateKey: kpOwner.input as string,
      wrappingPrivateKey: kpOwner.input as string,
      trusted: trustedFingerprintsStore()
    })

    // A folder back-fills descendants, so the same-role member is not skipped:
    // both Bob and the fresh Carol are POSTed.
    expect(envelopes.length).toBe(2)

    const bobEnvelope = (
      await Promise.all(
        envelopes.map(async (env) => ({
          env,
          forBob: await cryptfns.rsa
            .decryptMessage(kpBob, env.entries[0].encrypted_key)
            .then(() => true)
            .catch(() => false)
        }))
      )
    ).find((e) => e.forBob)!.env

    // The envelope must verify as a `grant` (before=null) against the owner's
    // pubkey — the canonical the server reconstructs for a same-role member; the
    // `role_change` canonical for the same move must fail.
    const grantInput = shareCrypto.buildAuditEventSigInput({
      senderId: OWNER_ID,
      recipientId: MEMBER_A,
      fileId: FOLDER_ID,
      action: 'grant',
      shareRoleBefore: null,
      shareRoleAfter: 'reader',
      timestamp: BigInt(bobEnvelope.member_signed_at as number)
    })
    expect(
      await shareCrypto.verifyAuditEvent(grantInput, bobEnvelope.event_signature, {
        pubkey: kpOwner.publicKey as string
      })
    ).toBe(true)

    const roleChangeInput = shareCrypto.buildAuditEventSigInput({
      senderId: OWNER_ID,
      recipientId: MEMBER_A,
      fileId: FOLDER_ID,
      action: 'role_change',
      shareRoleBefore: 'reader',
      shareRoleAfter: 'reader',
      timestamp: BigInt(bobEnvelope.member_signed_at as number)
    })
    expect(
      await shareCrypto.verifyAuditEvent(roleChangeInput, bobEnvelope.event_signature, {
        pubkey: kpOwner.publicKey as string
      })
    ).toBe(false)
  })

  it('share_to_group_signs_post_share_roster_for_folder_root', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    const fpBob = shareCrypto.computeFingerprint(kpBob.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      makeMemberWithKey(MEMBER_A, kpBob.publicKey as string, fpBob)
    ])

    const folderMembers: FolderMembersResponse = {
      folder_id: FOLDER_ID,
      folder_owner_id: OWNER_ID,
      folder_owner_pubkey_fingerprint: 'cc'.repeat(32),
      signature_algorithm: 'rsa-pss-sha256',
      members: [
        {
          user_id: OWNER_ID,
          email: 'owner@example.com',
          pubkey: kpOwner.publicKey as string,
          pubkey_fingerprint: 'cc'.repeat(32),
          share_role: 'co-owner',
          is_owner: true,
          added_at: 1,
          signed_by_user_id: OWNER_ID,
          member_signature: null
        }
      ],
      members_signed_at: 1,
      members_list_signature: 'sig',
      members_list_signed_by_user_id: OWNER_ID
    }
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(folderMembers)

    const envelopes: CreateShareEnvelope[] = []
    vi.spyOn(sharesApi, 'createShare').mockImplementation(async (envelope) => {
      envelopes.push(envelope)
      return { shares: [] }
    })

    await shareGroups.shareToGroup({
      groupId: GROUP_ID,
      root: { id: FOLDER_ID, user_id: OWNER_ID, mime: 'dir', encrypted_key: ownerWrap, is_owner: true } as never,
      subtree: [
        { id: FOLDER_ID, user_id: OWNER_ID, mime: 'dir', encrypted_key: ownerWrap, is_owner: true } as never,
        { id: CHILD_ID, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never
      ],
      shareRole: 'reader',
      senderId: OWNER_ID,
      privateKey: kpOwner.input as string,
      wrappingPrivateKey: kpOwner.input as string,
      trusted: trustedFingerprintsStore()
    })

    expect(envelopes.length).toBe(1)
    const sig = envelopes[0].members_list_signature
    expect(sig).toBeTruthy()
    expect(sig!.signed_by_user_id).toBe(OWNER_ID)

    // Reconstruct the exact post-share roster (owner + Bob under the caller)
    // and confirm the signature checks out — and that dropping Bob breaks it.
    const rosterInput = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        { userId: OWNER_ID, pubkeyFingerprintHex: 'cc'.repeat(32), shareRole: 'co-owner', isOwner: true, signedByUserId: OWNER_ID },
        { userId: MEMBER_A, pubkeyFingerprintHex: fpBob, shareRole: 'reader', isOwner: false, signedByUserId: OWNER_ID }
      ],
      membersSignedAt: BigInt(sig!.signed_at)
    })
    const rosterOk = await shareCrypto.verifyFolderMemberListSignature(
      rosterInput,
      sig!.signature,
      { pubkey: kpOwner.publicKey as string }
    )
    expect(rosterOk).toBe(true)

    const droppedBob = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: [
        { userId: OWNER_ID, pubkeyFingerprintHex: 'cc'.repeat(32), shareRole: 'co-owner', isOwner: true, signedByUserId: OWNER_ID }
      ],
      membersSignedAt: BigInt(sig!.signed_at)
    })
    const droppedOk = await shareCrypto.verifyFolderMemberListSignature(
      droppedBob,
      sig!.signature,
      { pubkey: kpOwner.publicKey as string }
    )
    expect(droppedOk).toBe(false)
  })

  it('share_to_group_rejects_a_group_with_no_other_recipients', async () => {
    const kp = await cryptfns.rsa.generateKeyPair()
    // The only roster row is the caller themselves — no one else to share with.
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      { user_id: OWNER_ID, email: 'owner@example.com', pubkey: kp.publicKey as string, fingerprint: shareCrypto.computeFingerprint(kp.publicKey as string), group_role: 'owner' }
    ])
    const submit = vi.spyOn(sharesApi, 'createShare').mockResolvedValue({ shares: [] })
    await expect(
      shareGroups.shareToGroup({
        groupId: GROUP_ID,
        root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: 'x', is_owner: true } as never,
        subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: 'x', is_owner: true } as never],
        shareRole: 'reader',
        senderId: OWNER_ID,
        privateKey: kp.input as string,
        wrappingPrivateKey: kp.input as string,
        trusted: trustedFingerprintsStore()
      })
    ).rejects.toThrow(/no one else/i)
    expect(submit).not.toHaveBeenCalled()
  })

  it('share_to_group_hard_fails_when_server_lies_about_member_pubkey', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpHonest = await cryptfns.rsa.generateKeyPair()
    // The attacker substitutes its own key but keeps the honest member's
    // claimed fingerprint — exactly the substitution the binding check
    // exists to catch.
    const kpAttacker = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    const honestFingerprint = shareCrypto.computeFingerprint(kpHonest.publicKey as string)
    const lyingRoster: GroupMemberWithKey[] = [
      {
        user_id: MEMBER_A,
        email: 'bob@example.com',
        pubkey: kpAttacker.publicKey as string,
        fingerprint: honestFingerprint,
        group_role: 'reader'
      }
    ]
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue(lyingRoster)
    const submit = vi.spyOn(sharesApi, 'createShare').mockResolvedValue({ shares: [] })
    const wrapSpy = vi.spyOn(shareCrypto, 'wrapForRecipient')

    await expect(
      shareGroups.shareToGroup({
        groupId: GROUP_ID,
        root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never,
        subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never],
        shareRole: 'editor',
        senderId: OWNER_ID,
        privateKey: kpOwner.input as string,
        wrappingPrivateKey: kpOwner.input as string,
        trusted: trustedFingerprintsStore()
      })
    ).rejects.toBeInstanceOf(GroupMemberFingerprintMismatch)

    // No key was wrapped under the attacker's pubkey and nothing was POSTed —
    // the fan-out aborted before any of it.
    expect(wrapSpy).not.toHaveBeenCalled()
    expect(submit).not.toHaveBeenCalled()
  })

  it('share_to_group_hard_fails_when_member_key_changed_since_trusted', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpRotated = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    // The caller trusted a different fingerprint for this member earlier; the
    // roster now ships a self-consistent but unrecognized key. TOFU treats
    // that as a key rotation and hard-stops.
    const trusted = trustedFingerprintsStore()
    trusted.bind(OWNER_ID)
    trusted.trustFingerprint(MEMBER_A, 'dd'.repeat(32), 'in-person')

    const member = makeMemberWithKey(MEMBER_A, kpRotated.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([member])
    const submit = vi.spyOn(sharesApi, 'createShare').mockResolvedValue({ shares: [] })
    const wrapSpy = vi.spyOn(shareCrypto, 'wrapForRecipient')

    await expect(
      shareGroups.shareToGroup({
        groupId: GROUP_ID,
        root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never,
        subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never],
        shareRole: 'editor',
        senderId: OWNER_ID,
        privateKey: kpOwner.input as string,
        wrappingPrivateKey: kpOwner.input as string,
        trusted
      })
    ).rejects.toBeInstanceOf(GroupMemberFingerprintMismatch)
    expect(wrapSpy).not.toHaveBeenCalled()
    expect(submit).not.toHaveBeenCalled()
  })

  it('share_to_group_rejects_fabricated_key_transition_rows', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpRotated = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    const trustedFp = 'dd'.repeat(32)
    const trusted = trustedFingerprintsStore()
    trusted.bind(OWNER_ID)
    trusted.trustFingerprint(MEMBER_A, trustedFp, 'in-person')

    const member = makeMemberWithKey(MEMBER_A, kpRotated.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([member])
    // A hostile server "explains" the key change with a transition row whose
    // fingerprints link the pinned fingerprint to the new key but whose
    // certificate carries no valid signatures. Linkage alone must not re-pin.
    vi.spyOn(sharesApi, 'getKeyTransitions').mockResolvedValue([
      {
        user_id: MEMBER_A,
        old_fingerprint: trustedFp,
        old_key_spki: [1, 2, 3],
        old_key_type: 'rsa',
        new_fingerprint: member.fingerprint,
        new_identity_key_pem: 'not-a-key',
        new_wrapping_key_pem: 'not-a-key',
        old_signature: [4, 5, 6],
        new_signature: [7, 8, 9],
        issued_at: 1_700_000_000
      }
    ])
    const submit = vi.spyOn(sharesApi, 'createShare').mockResolvedValue({ shares: [] })
    const wrapSpy = vi.spyOn(shareCrypto, 'wrapForRecipient')

    await expect(
      shareGroups.shareToGroup({
        groupId: GROUP_ID,
        root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never,
        subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never],
        shareRole: 'editor',
        senderId: OWNER_ID,
        privateKey: kpOwner.input as string,
        wrappingPrivateKey: kpOwner.input as string,
        trusted
      })
    ).rejects.toBeInstanceOf(GroupMemberFingerprintMismatch)
    expect(trusted.lookup(MEMBER_A)?.pubkeyFingerprint).toBe(trustedFp)
    expect(wrapSpy).not.toHaveBeenCalled()
    expect(submit).not.toHaveBeenCalled()
  })

  it('share_to_group_records_first_sight_fingerprints', async () => {
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const ownerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpOwner.publicKey as string
    )

    const trusted = trustedFingerprintsStore()
    trusted.bind(OWNER_ID)
    const member = makeMemberWithKey(MEMBER_A, kpBob.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([member])
    vi.spyOn(sharesApi, 'createShare').mockResolvedValue({ shares: [] })

    await shareGroups.shareToGroup({
      groupId: GROUP_ID,
      root: { id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never,
      subtree: [{ id: FILE_X, user_id: OWNER_ID, mime: 'application/pdf', encrypted_key: ownerWrap, is_owner: true } as never],
      shareRole: 'editor',
      senderId: OWNER_ID,
      privateKey: kpOwner.input as string,
      wrappingPrivateKey: kpOwner.input as string,
      trusted
    })

    // First sight of a member records the recomputed fingerprint under TOFU so
    // a subsequent silent substitution is caught.
    expect(trusted.lookup(MEMBER_A)?.pubkeyFingerprint).toBe(
      shareCrypto.computeFingerprint(kpBob.publicKey as string)
    )
  })
})

describe('share dialog group target routing', () => {
  function mountDialog(callerWrap: string, kpAlice: { input: string; publicKey: string }) {
    return mount(SharingPeopleAdd, {
      props: {
        file: {
          id: FILE_X,
          user_id: OWNER_ID,
          is_owner: true,
          name: 'doc.pdf',
          name_hash: '',
          mime: 'application/pdf',
          chunks: 0,
          file_id: null,
          file_modified_at: 0,
          created_at: 0,
          is_new: false,
          editable: false,
          active_version: 1,
          encrypted_key: callerWrap,
          encrypted_name: '',
          cipher: 'aegis128l'
        } as unknown as Parameters<typeof SharingPeopleAdd>[0]['file'],
        authenticatedUserId: OWNER_ID,
        keypair: {
          input: kpAlice.input,
          publicKey: kpAlice.publicKey
        } as unknown as Parameters<typeof SharingPeopleAdd>[0]['keypair']
      }
    })
  }

  it('group_pick_fans_out_through_create_share_per_member', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    const kpCarol = await cryptfns.rsa.generateKeyPair()

    const fileKey = await cryptfns.aes.generateKey()
    const fileKeyHex = cryptfns.uint8.toHex(fileKey)
    const callerWrap = await cryptfns.rsa.encryptMessage(fileKeyHex, kpAlice.publicKey as string)

    capabilitiesStore().caps = enabledCapabilities()

    const fpBob = shareCrypto.computeFingerprint(kpBob.publicKey as string)
    const fpCarol = shareCrypto.computeFingerprint(kpCarol.publicKey as string)
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      { user_id: MEMBER_A, email: 'bob@example.com', pubkey: kpBob.publicKey as string, fingerprint: fpBob, group_role: 'reader' },
      { user_id: MEMBER_B, email: 'carol@example.com', pubkey: kpCarol.publicKey as string, fingerprint: fpCarol, group_role: 'editor' }
    ])
    vi.spyOn(sharesApi, 'getShareRecipients').mockResolvedValue([])
    // The dialog gates submit only on a known-zero owned roster, so seed a
    // group with members so the Share button is enabled.
    const groupWithMembers = makeGroup({
      members: [
        { user_id: MEMBER_A, email: 'bob@example.com', fingerprint: fpBob, added_at: 1, group_role: 'reader' },
        { user_id: MEMBER_B, email: 'carol@example.com', fingerprint: fpCarol, added_at: 2, group_role: 'editor' }
      ]
    })
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({ owned: [groupWithMembers], member_of: [] })
    const createShareSpy = vi.spyOn(sharesApi, 'createShare').mockResolvedValue({ shares: [] })

    const wrapper = mountDialog(callerWrap, {
      input: kpAlice.input as string,
      publicKey: kpAlice.publicKey as string
    })
    await flushPromises()

    await wrapper.get('input[name="recipient-email"]').setValue('Marketing')
    await wrapper.get('[data-testid="share-dialog-discover"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="share-dialog-group-panel"]').exists()).toBe(true)
    expect(
      (wrapper.find('[data-testid="share-dialog-submit"]').element as HTMLButtonElement).disabled
    ).toBe(false)

    await wrapper.get('[data-testid="share-dialog-submit"]').trigger('click')
    for (let i = 0; i < 100 && createShareSpy.mock.calls.length < 2; i++) {
      await flushPromises()
      await new Promise((resolve) => setTimeout(resolve, 0))
    }
    // The fan-out POSTs one share per member through the standard single-share
    // endpoint — no dedicated group-shares endpoint.
    expect(createShareSpy).toHaveBeenCalledTimes(2)
    for (const [envelope] of createShareSpy.mock.calls) {
      expect(envelope.entries[0].file_id).toBe(FILE_X)
    }
  })

  it('group_picker_includes_editor_and_co_owner_member_of_groups_only', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    const fileKey = await cryptfns.aes.generateKey()
    const callerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpAlice.publicKey as string
    )
    capabilitiesStore().caps = enabledCapabilities()
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({
      owned: [makeGroup({ name: 'Owned' })],
      member_of: [
        { id: 'g-ed', owner_id: MEMBER_A, owner_email: 'a@x.com', name: 'EditorGroup', created_at: 1, added_at: 1, group_role: 'editor' },
        { id: 'g-co', owner_id: MEMBER_A, owner_email: 'a@x.com', name: 'CoOwnerGroup', created_at: 1, added_at: 1, group_role: 'co-owner' },
        { id: 'g-rd', owner_id: MEMBER_A, owner_email: 'a@x.com', name: 'ReaderGroup', created_at: 1, added_at: 1, group_role: 'reader' }
      ]
    })

    const wrapper = mountDialog(callerWrap, {
      input: kpAlice.input as string,
      publicKey: kpAlice.publicKey as string
    })
    await flushPromises()
    const html = wrapper.html()
    expect(html).toContain('Owned')
    expect(html).toContain('EditorGroup')
    expect(html).toContain('CoOwnerGroup')
    // A reader cannot initiate a share, so a reader member-of group is not
    // offered as a target.
    expect(html).not.toContain('ReaderGroup')
  })

  it('group_picker_empty_when_share_groups_disabled', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    const fileKey = await cryptfns.aes.generateKey()
    const callerWrap = await cryptfns.rsa.encryptMessage(
      cryptfns.uint8.toHex(fileKey),
      kpAlice.publicKey as string
    )
    capabilitiesStore().caps = enabledCapabilities({ share_groups: false })
    const listSpy = vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({
      owned: [makeGroup({ name: 'Owned' })],
      member_of: []
    })

    const wrapper = mountDialog(callerWrap, {
      input: kpAlice.input as string,
      publicKey: kpAlice.publicKey as string
    })
    await flushPromises()
    // With groups disabled the picker never renders — and the dialog never
    // even reaches for the group list.
    expect(wrapper.find('[data-testid="share-dialog-group-suggestions"]').exists()).toBe(false)
    expect(listSpy).not.toHaveBeenCalled()
  })
})

describe('group role UI', () => {
  function mountHub(kp: { input: string; publicKey: string }) {
    return mount(ShareHubGroups, {
      props: {
        authenticated: { user: { id: OWNER_ID, email: 'alice@example.com' } }
      },
      attrs: {
        keypair: { input: kp.input, publicKey: kp.publicKey }
      }
    })
  }

  it('owned_group_owner_can_set_any_member_role', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    capabilitiesStore().caps = enabledCapabilities()
    const group = makeGroup({
      members: [
        { user_id: MEMBER_A, email: 'bob@example.com', fingerprint: 'fp-bob', added_at: 1, group_role: 'reader' }
      ]
    })
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({ owned: [group], member_of: [] })
    const setRole = vi.spyOn(sharesApi, 'setGroupMemberRole').mockResolvedValue(undefined)

    const wrapper = mountHub({ input: kpAlice.input as string, publicKey: kpAlice.publicKey as string })
    await flushPromises()

    const select = wrapper.get(
      `[data-testid="share-hub-groups-owned-${GROUP_ID}-member-${MEMBER_A}-role"]`
    )
    // The owner's role picker offers all three group roles, co-owner included.
    const optionValues = select.findAll('option').map((o) => (o.element as HTMLOptionElement).value)
    expect(optionValues).toEqual(['reader', 'editor', 'co-owner'])

    await select.setValue('co-owner')
    await flushPromises()
    expect(setRole).toHaveBeenCalledWith(GROUP_ID, MEMBER_A, 'co-owner')
  })

  it('member_of_editor_sees_share_hint_not_manage', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    capabilitiesStore().caps = enabledCapabilities()
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({
      owned: [],
      member_of: [
        { id: 'g-ed', owner_id: MEMBER_A, owner_email: 'a@x.com', name: 'EditorGroup', created_at: 1, added_at: 1, group_role: 'editor' }
      ]
    })
    const wrapper = mountHub({ input: kpAlice.input as string, publicKey: kpAlice.publicKey as string })
    await flushPromises()
    expect(wrapper.find('[data-testid="share-hub-groups-member-of-g-ed-editor-hint"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="share-hub-groups-member-of-g-ed-manage"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="share-hub-groups-member-of-g-ed-add"]').exists()).toBe(false)
  })

  it('member_of_co_owner_gets_manage_controls_no_delete', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    capabilitiesStore().caps = enabledCapabilities()
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({
      owned: [],
      member_of: [
        { id: 'g-co', owner_id: MEMBER_A, owner_email: 'a@x.com', name: 'CoGroup', created_at: 1, added_at: 1, group_role: 'co-owner' }
      ]
    })
    const wrapper = mountHub({ input: kpAlice.input as string, publicKey: kpAlice.publicKey as string })
    await flushPromises()
    expect(wrapper.find('[data-testid="share-hub-groups-member-of-g-co-manage"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="share-hub-groups-member-of-g-co-add"]').exists()).toBe(true)
    // Rename and delete are owner-only — a co-owner has neither affordance.
    expect(wrapper.find('[data-testid="share-hub-groups-member-of-g-co-rename"]').exists()).toBe(false)
    expect(wrapper.html()).not.toContain('share-hub-groups-member-of-g-co-delete')
  })

  it('co_owner_managed_roster_excludes_the_owner_row', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    const kpOwner = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    capabilitiesStore().caps = enabledCapabilities()
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({
      owned: [],
      member_of: [
        { id: 'g-co', owner_id: MEMBER_B, owner_email: 'owner@example.com', name: 'CoGroup', created_at: 1, added_at: 1, group_role: 'co-owner' }
      ]
    })
    vi.spyOn(sharesApi, 'groupMembers').mockResolvedValue([
      { user_id: MEMBER_B, email: 'owner@example.com', pubkey: kpOwner.publicKey as string, fingerprint: shareCrypto.computeFingerprint(kpOwner.publicKey as string), group_role: 'owner' },
      { user_id: MEMBER_A, email: 'bob@example.com', pubkey: kpBob.publicKey as string, fingerprint: shareCrypto.computeFingerprint(kpBob.publicKey as string), group_role: 'reader' }
    ])
    const wrapper = mountHub({ input: kpAlice.input as string, publicKey: kpAlice.publicKey as string })
    await flushPromises()

    await wrapper.get('[data-testid="share-hub-groups-member-of-g-co-load-roster"]').trigger('click')
    await flushPromises()

    // The owner row is never rendered as a manageable member; only Bob is.
    expect(wrapper.find(`[data-testid="share-hub-groups-member-of-g-co-member-${MEMBER_A}"]`).exists()).toBe(true)
    expect(wrapper.find(`[data-testid="share-hub-groups-member-of-g-co-member-${MEMBER_B}"]`).exists()).toBe(false)
  })

  it('owned_group_add_dialog_offers_co_owner_to_owner', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    capabilitiesStore().caps = enabledCapabilities()
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({ owned: [makeGroup()], member_of: [] })
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: MEMBER_A,
      email: 'bob@example.com',
      pubkey: kpBob.publicKey as string,
      fingerprint: shareCrypto.computeFingerprint(kpBob.publicKey as string)
    })

    const wrapper = mountHub({ input: kpAlice.input as string, publicKey: kpAlice.publicKey as string })
    await flushPromises()

    await wrapper.get(`[data-testid="share-hub-groups-owned-${GROUP_ID}-add"]`).trigger('click')
    await flushPromises()
    await wrapper.get('input[name="group-add-email"]').setValue('bob@example.com')
    await wrapper.get('[data-testid="group-add-member-discover"]').trigger('click')
    await flushPromises()

    // The owner of the group may grant co-owner, so the role option renders.
    expect(wrapper.find('[data-testid="group-add-member-role-coowner"]').exists()).toBe(true)
  })

  it('co_owner_managed_group_add_dialog_hides_co_owner_role', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    const kpBob = await cryptfns.rsa.generateKeyPair()
    capabilitiesStore().caps = enabledCapabilities()
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({
      owned: [],
      member_of: [
        {
          id: 'g-co',
          owner_id: MEMBER_B,
          owner_email: 'owner@example.com',
          name: 'CoGroup',
          created_at: 1,
          added_at: 1,
          group_role: 'co-owner'
        }
      ]
    })
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: MEMBER_A,
      email: 'bob@example.com',
      pubkey: kpBob.publicKey as string,
      fingerprint: shareCrypto.computeFingerprint(kpBob.publicKey as string)
    })

    const wrapper = mountHub({ input: kpAlice.input as string, publicKey: kpAlice.publicKey as string })
    await flushPromises()

    await wrapper.get('[data-testid="share-hub-groups-member-of-g-co-add"]').trigger('click')
    await flushPromises()
    await wrapper.get('input[name="group-add-email"]').setValue('bob@example.com')
    await wrapper.get('[data-testid="group-add-member-discover"]').trigger('click')
    await flushPromises()

    // A group co-owner who is not the owner may add reader/editor but never
    // another co-owner — the option must not render.
    expect(wrapper.find('[data-testid="group-add-member-role-editor"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="group-add-member-role-coowner"]').exists()).toBe(false)
  })

  it('group_role_hidden_when_share_groups_disabled', async () => {
    const kpAlice = await cryptfns.rsa.generateKeyPair()
    const caps = capabilitiesStore()
    caps.caps = enabledCapabilities({ share_groups: false })
    // Mark the advertisement as already fetched so the component's mount-time
    // capability refresh doesn't fire and overwrite the seeded value.
    caps.lastFetchedAt = Math.floor(Date.now() / 1000)
    const group = makeGroup({
      members: [
        { user_id: MEMBER_A, email: 'bob@example.com', fingerprint: 'fp-bob', added_at: 1, group_role: 'reader' }
      ]
    })
    vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({ owned: [group], member_of: [] })
    const wrapper = mountHub({ input: kpAlice.input as string, publicKey: kpAlice.publicKey as string })
    await flushPromises()
    // No role <select> renders when the server doesn't advertise groups —
    // fail-closed to a read-only label.
    expect(
      wrapper.find(`[data-testid="share-hub-groups-owned-${GROUP_ID}-member-${MEMBER_A}-role"]`).exists()
    ).toBe(false)
    expect(
      wrapper.find(`[data-testid="share-hub-groups-owned-${GROUP_ID}-member-${MEMBER_A}-role-label"]`).exists()
    ).toBe(true)
    // The rename affordance is also gated on the capability.
    expect(wrapper.find(`[data-testid="share-hub-groups-owned-${GROUP_ID}-rename"]`).exists()).toBe(false)
  })
})
