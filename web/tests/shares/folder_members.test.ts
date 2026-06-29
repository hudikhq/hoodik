import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'

import * as cryptfns from '../../services/cryptfns'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'
import FolderMembersView from '../../src/components/shares/FolderMembersView.vue'

import type {
  AppFile,
  AppShare,
  FolderMember,
  FolderMembersResponse,
  KeyPair
} from '../../types'

const FOLDER_ID = '11111111-1111-1111-1111-111111111111'
const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
const EDITOR_ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
const COOWNER_ID = 'cccccccc-cccc-cccc-cccc-cccccccccccc'
const RESHARED_ID = 'dddddddd-dddd-dddd-dddd-dddddddddddd'

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
  role: FolderMember['share_role']
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

async function buildSignedMember(
  userId: string,
  email: string | null,
  role: FolderMember['share_role'],
  signerPrivateKey: string,
  signedByUserId: string,
  isOwner = false
): Promise<{ member: FolderMember; kp: KeyPair }> {
  const kp = await cryptfns.rsa.generateKeyPair()
  const fingerprint = shareCrypto.computeFingerprint(kp.publicKey as string)
  const addedAt = 1_700_000_000
  const sig = await signMember(
    kp.publicKey as string,
    userId,
    signerPrivateKey,
    addedAt,
    role
  )
  return {
    member: {
      user_id: userId,
      email,
      pubkey: kp.publicKey as string,
      pubkey_fingerprint: fingerprint,
      share_role: role,
      is_owner: isOwner,
      added_at: addedAt,
      signed_by_user_id: signedByUserId,
      member_signature: sig
    },
    kp
  }
}

function buildFolderFixture(): AppFile {
  return {
    id: FOLDER_ID,
    user_id: OWNER_ID,
    is_owner: true,
    name: 'shared-folder',
    name_hash: '',
    mime: 'dir',
    chunks: 0,
    file_id: null,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: 'aegis128l'
  } as unknown as AppFile
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('FolderMembersView', () => {
  // The view's revoke handler builds a fresh list signature with the
  // caller's private key. Sharing one keypair across every test lets
  // them sign without having to thread the kp through the spy mocks.
  let viewKeypair: KeyPair | null = null

  async function buildSignedResponse(): Promise<{
    response: FolderMembersResponse
    keypairs: Map<string, KeyPair>
  }> {
    // Use a stable keypair across tests so the view's signing path can
    // run end-to-end without each test having to mock signFolderMemberList.
    if (!viewKeypair) {
      viewKeypair = await cryptfns.rsa.generateKeyPair()
    }
    const ownerKp = viewKeypair
    const ownerSig = await signMember(
      ownerKp.publicKey as string,
      OWNER_ID,
      ownerKp.input as string,
      1_700_000_000,
      'co-owner'
    )
    const ownerMember: FolderMember = {
      user_id: OWNER_ID,
      email: 'alice@example.com',
      pubkey: ownerKp.publicKey as string,
      pubkey_fingerprint: shareCrypto.computeFingerprint(ownerKp.publicKey as string),
      share_role: 'co-owner',
      is_owner: true,
      added_at: 1_700_000_000,
      signed_by_user_id: OWNER_ID,
      member_signature: ownerSig
    }
    const editor = await buildSignedMember(
      EDITOR_ID,
      'bob@example.com',
      'editor',
      ownerKp.input as string,
      OWNER_ID
    )
    const coowner = await buildSignedMember(
      COOWNER_ID,
      'carol@example.com',
      'co-owner',
      ownerKp.input as string,
      OWNER_ID
    )
    const reshared = await buildSignedMember(
      RESHARED_ID,
      'dan@example.com',
      'reader',
      coowner.kp.input as string,
      COOWNER_ID
    )
    const allMembers: FolderMember[] = [ownerMember, editor.member, coowner.member, reshared.member]
    const signedAt = 1_700_000_500
    const listInput = shareCrypto.buildFolderMemberListInput({
      folderId: FOLDER_ID,
      folderOwnerId: OWNER_ID,
      members: allMembers.map((m) => ({
        userId: m.user_id,
        pubkeyFingerprintHex: m.pubkey_fingerprint,
        shareRole: m.share_role,
        isOwner: m.is_owner,
        signedByUserId: m.signed_by_user_id ?? OWNER_ID
      })),
      membersSignedAt: BigInt(signedAt)
    })
    const { signature: listSig } = await shareCrypto.signFolderMemberList(
      listInput,
      ownerKp.input as string
    )
    const response: FolderMembersResponse = {
      folder_id: FOLDER_ID,
      folder_owner_id: OWNER_ID,
      folder_owner_pubkey_fingerprint: ownerMember.pubkey_fingerprint,
      signature_algorithm: 'rsa-pss-sha256',
      members: allMembers,
      members_signed_at: signedAt,
      members_list_signature: listSig,
      members_list_signed_by_user_id: OWNER_ID
    }
    const keypairs = new Map<string, KeyPair>([
      [OWNER_ID, ownerKp],
      [EDITOR_ID, editor.kp],
      [COOWNER_ID, coowner.kp],
      [RESHARED_ID, reshared.kp]
    ])
    return { response, keypairs }
  }

  function mountView(
    response: FolderMembersResponse,
    outgoingShares: AppShare[] = [],
    authenticatedUserId: string = OWNER_ID
  ): ReturnType<typeof mount> {
    return mount(FolderMembersView, {
      props: {
        folder: buildFolderFixture(),
        authenticatedUserId,
        keypair: viewKeypair as KeyPair,
        outgoingShares
      }
    })
  }

  it('folder_members_view_renders_each_member_with_role_badge', async () => {
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response)
    await flushPromises()
    for (const member of response.members) {
      expect(
        wrapper.find(`[data-testid="folder-members-view-row-${member.user_id}"]`).exists()
      ).toBe(true)
      const roleBadge = wrapper.find(
        `[data-testid="folder-members-view-row-${member.user_id}-role"]`
      )
      if (member.is_owner) {
        // The owner's authority is conveyed by the "owner" badge alone;
        // pairing it with a "co-owner" role pill reads as a contradiction.
        expect(roleBadge.exists()).toBe(false)
        expect(
          wrapper
            .find(`[data-testid="folder-members-view-row-${member.user_id}-owner-badge"]`)
            .exists()
        ).toBe(true)
      } else {
        expect(roleBadge.text()).toContain(member.share_role)
      }
    }
  })

  it('folder_members_view_shows_added_by_owner_vs_co_owner', async () => {
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response)
    await flushPromises()
    const editorRow = wrapper.find(
      `[data-testid="folder-members-view-row-${EDITOR_ID}-added-by"]`
    )
    const resharedRow = wrapper.find(
      `[data-testid="folder-members-view-row-${RESHARED_ID}-added-by"]`
    )
    expect(editorRow.text()).toContain('owner')
    expect(resharedRow.text()).toContain('Co-owner')
  })

  it('folder_members_view_signature_badge_verified_on_valid_sig', async () => {
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response)
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${EDITOR_ID}-sig-verified"]`).exists()
    ).toBe(true)
  })

  it('folder_members_view_owner_row_hides_signature_badge', async () => {
    // The owner attests to the list directly; rendering a row-level
    // signature badge under their entry duplicates the "owner" badge
    // already on the role line and reads as a bug to non-technical users.
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response)
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${OWNER_ID}-sig-verified"]`).exists()
    ).toBe(false)
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${OWNER_ID}-sig-unsigned"]`).exists()
    ).toBe(false)
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${OWNER_ID}-owner-badge"]`).exists()
    ).toBe(true)
  })

  it('folder_members_view_marks_unsigned_member_verified_when_list_signed', async () => {
    // The list signature authenticates every named member; a missing
    // per-row `member_signature` is bookkeeping noise rather than a
    // legitimacy gap, so the row stays verified.
    const { response } = await buildSignedResponse()
    const noPerRowSig: FolderMembersResponse = {
      ...response,
      members: response.members.map((m) =>
        m.user_id === EDITOR_ID ? { ...m, member_signature: null } : m
      )
    }
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(noPerRowSig)
    const wrapper = mountView(noPerRowSig)
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${EDITOR_ID}-sig-verified"]`).exists()
    ).toBe(true)
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${EDITOR_ID}-sig-unsigned"]`).exists()
    ).toBe(false)
  })

  it('folder_members_view_signature_badge_failed_on_tampered_sig', async () => {
    const { response } = await buildSignedResponse()
    const tampered: FolderMembersResponse = {
      ...response,
      members: response.members.map((m) =>
        m.user_id === EDITOR_ID
          ? {
              ...m,
              member_signature:
                'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'
            }
          : m
      )
    }
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(tampered)
    const wrapper = mountView(tampered)
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${EDITOR_ID}-sig-failed"]`).exists()
    ).toBe(true)
  })

  it('folder_members_view_revoke_calls_delete_endpoint', async () => {
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const revokeSpy = vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
    const signSpy = vi
      .spyOn(shareCrypto, 'signAuditEvent')
      .mockResolvedValue('sig-stub')
    const wrapper = mountView(response, [])
    await flushPromises()
    await wrapper
      .find(`[data-testid="folder-members-view-row-${EDITOR_ID}-revoke"]`)
      .trigger('click')
    await flushPromises()
    // Click opens the confirmation modal; the endpoint stays untouched
    // until the user accepts the force-rotate disclaimer.
    expect(wrapper.find('[data-testid="revoke-confirm-modal"]').exists()).toBe(true)
    expect(revokeSpy).not.toHaveBeenCalled()
    await wrapper
      .find('[data-testid="revoke-confirm-modal-accept"]')
      .trigger('click')
    await flushPromises()
    expect(signSpy).toHaveBeenCalled()
    expect(revokeSpy).toHaveBeenCalledWith(
      FOLDER_ID,
      EDITOR_ID,
      expect.objectContaining({ event_signature: 'sig-stub' })
    )
  })

  it('folder_members_view_hides_revoke_button_for_non_owner_caller', async () => {
    // Editors / Readers see the roster but cannot revoke. The revoke
    // affordance disappears entirely; the mutating endpoint would 401
    // for non-owners anyway, so we never surface a button that toasts.
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response, [], EDITOR_ID)
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${EDITOR_ID}"]`).exists()
    ).toBe(true)
    for (const id of [EDITOR_ID, COOWNER_ID, RESHARED_ID]) {
      expect(
        wrapper.find(`[data-testid="folder-members-view-row-${id}-revoke"]`).exists()
      ).toBe(false)
    }
  })

  it('folder_members_view_renders_change_button_alongside_revoke', async () => {
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response)
    await flushPromises()
    for (const id of [EDITOR_ID, COOWNER_ID, RESHARED_ID]) {
      expect(
        wrapper.find(`[data-testid="folder-members-view-row-${id}-change"]`).exists()
      ).toBe(true)
      expect(
        wrapper.find(`[data-testid="folder-members-view-row-${id}-revoke"]`).exists()
      ).toBe(true)
    }
    // Owner row stays single-affordance — it has neither change nor revoke.
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${OWNER_ID}-change"]`).exists()
    ).toBe(false)
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${OWNER_ID}-revoke"]`).exists()
    ).toBe(false)
  })

  it('folder_members_view_change_emits_change_role_with_email_and_role', async () => {
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response)
    await flushPromises()
    await wrapper
      .find(`[data-testid="folder-members-view-row-${EDITOR_ID}-change"]`)
      .trigger('click')
    const events = wrapper.emitted('change-role')
    expect(events).toBeTruthy()
    expect(events?.[0]).toEqual([{ email: 'bob@example.com', role: 'editor' }])
  })

  it('folder_members_view_shows_upload_hint_on_editor_and_co_owner_rows', async () => {
    // The role itself implies the upload right — we
    // surface a small "+upload" hint next to the role badge so a glance
    // at the roster answers the obvious question.
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response)
    await flushPromises()
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${EDITOR_ID}-can-upload"]`).exists()
    ).toBe(true)
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${COOWNER_ID}-can-upload"]`).exists()
    ).toBe(true)
    // Reader has no upload right — the hint stays hidden.
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${RESHARED_ID}-can-upload"]`).exists()
    ).toBe(false)
    // Owner row carries the "owner" badge instead; upload hint stays off.
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${OWNER_ID}-can-upload"]`).exists()
    ).toBe(false)
  })

  it('folder_members_view_change_disabled_when_email_unknown', async () => {
    // A row that lacks a discoverable email (rare — id-only fallback in
    // pre-1.16 shares) can't be re-shared by email; the Change button
    // stays disabled rather than emitting a useless event.
    const { response } = await buildSignedResponse()
    const noEmailResponse: FolderMembersResponse = {
      ...response,
      members: response.members.map((m) =>
        m.user_id === EDITOR_ID ? { ...m, email: null } : m
      )
    }
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(noEmailResponse)
    const wrapper = mountView(noEmailResponse)
    await flushPromises()
    const btn = wrapper.find(
      `[data-testid="folder-members-view-row-${EDITOR_ID}-change"]`
    )
    expect(btn.exists()).toBe(true)
    expect((btn.element as HTMLButtonElement).disabled).toBe(true)
  })

  it('folder_members_view_revoke_co_owner_shows_cascade_confirmation', async () => {
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const revokeSpy = vi.spyOn(sharesApi, 'revokeShare').mockResolvedValue(undefined)
    vi.spyOn(shareCrypto, 'signAuditEvent').mockResolvedValue('sig-stub')

    // One outgoing share was issued by the Co-owner — the cascade
    // confirmation line in the modal must surface that count.
    const outgoing: AppShare[] = [
      {
        file_id: FOLDER_ID,
        recipient_id: RESHARED_ID,
        recipient_email: 'dan@example.com',
        recipient_pubkey_fingerprint: 'fp',
        share_role: 'reader',
        created_at: 1_700_000_500,
        shared_at: 1_700_000_500,
        shared_by_user_id: COOWNER_ID,
        shared_by_email: 'carol@example.com'
      }
    ]
    const wrapper = mountView(response, outgoing)
    await flushPromises()
    await wrapper
      .find(`[data-testid="folder-members-view-row-${COOWNER_ID}-revoke"]`)
      .trigger('click')
    await flushPromises()
    // Modal visible, cascade line rendered.
    expect(wrapper.find('[data-testid="revoke-confirm-modal"]').exists()).toBe(true)
    expect(
      wrapper.find('[data-testid="revoke-confirm-modal-cascade"]').exists()
    ).toBe(true)
    expect(
      wrapper.find('[data-testid="revoke-confirm-modal-cascade"]').text()
    ).toContain('1')
    expect(revokeSpy).not.toHaveBeenCalled()
    await wrapper
      .find('[data-testid="revoke-confirm-modal-accept"]')
      .trigger('click')
    await flushPromises()
    expect(revokeSpy).toHaveBeenCalledWith(
      FOLDER_ID,
      COOWNER_ID,
      expect.any(Object)
    )
  })

  it('folder_members_view_co_owner_sees_change_and_revoke_on_peer_rows', async () => {
    // A Co-owner has peer rights with the owner — the
    // change + revoke buttons surface on every non-owner row except
    // the Co-owner's own. Previously the affordance was gated on
    // `isOwner`, which hid it from Co-owners entirely.
    const { response } = await buildSignedResponse()
    vi.spyOn(sharesApi, 'getFolderMembers').mockResolvedValue(response)
    const wrapper = mountView(response, [], COOWNER_ID)
    await flushPromises()
    for (const id of [EDITOR_ID, RESHARED_ID]) {
      expect(
        wrapper.find(`[data-testid="folder-members-view-row-${id}-change"]`).exists()
      ).toBe(true)
      expect(
        wrapper.find(`[data-testid="folder-members-view-row-${id}-revoke"]`).exists()
      ).toBe(true)
    }
    // Owner row never gets controls; the Co-owner's own row stays
    // controlless to avoid the self-revoke confusion (self-remove
    // goes through the Shared-with-me leave flow instead).
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${OWNER_ID}-change"]`).exists()
    ).toBe(false)
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${COOWNER_ID}-change"]`).exists()
    ).toBe(false)
    expect(
      wrapper.find(`[data-testid="folder-members-view-row-${COOWNER_ID}-revoke"]`).exists()
    ).toBe(false)
  })
})
