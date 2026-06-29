<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  mdiCheckCircle,
  mdiAlertCircle,
  mdiHelpCircleOutline,
  mdiTrashCan
} from '@mdi/js'

import BaseIcon from '@/components/ui/BaseIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import RevokeConfirmModal from '@/components/shares/RevokeConfirmModal.vue'
import { api as sharesApi, crypto as shareCrypto, editable as editableSvc } from '!/shares'
import { errorNotification, notification } from '!/index'

import type {
  AppFile,
  AppShare,
  FolderMember,
  FolderMembersResponse,
  KeyPair,
  ShareRole
} from 'types'

const props = defineProps<{
  folder: AppFile
  authenticatedUserId: string
  keypair: KeyPair
  /** Outgoing shares the owner has issued on this folder — used to count
   * Co-owner cascade impact on revoke. */
  outgoingShares: AppShare[]
}>()

const emit = defineEmits<{
  (e: 'changed'): void
  (e: 'change-role', payload: { email: string; role: ShareRole }): void
}>()

const response = ref<FolderMembersResponse | null>(null)
const signatureStatus = ref<Record<string, 'verified' | 'failed' | 'unsigned'>>({})
const loading = ref(false)
const loadError = ref<string | null>(null)
const revokeConfirm = ref<{
  member: FolderMember
  affectedCount: number
} | null>(null)

onMounted(async () => {
  await refresh()
})

async function refresh(): Promise<void> {
  loading.value = true
  loadError.value = null
  try {
    response.value = await sharesApi.getFolderMembers(props.folder.id)
    await verifySignatures(response.value)
  } catch (err) {
    loadError.value = (err as Error).message || 'Failed to load folder members'
  } finally {
    loading.value = false
  }
}

/**
 * The per-row badge reflects whether the member's presence in the list
 * is covered by a verified `FolderMemberListV1` signature. The list
 * signature is the primary authentication — every member named in a
 * list whose signature verifies is itself verified. Truly legacy rows
 * (shares from before this protocol existed) are the only case where we
 * surface "no signature".
 */
async function verifySignatures(payload: FolderMembersResponse): Promise<void> {
  const statuses: Record<string, 'verified' | 'failed' | 'unsigned'> = {}
  const hasListSignature = Boolean(payload.members_list_signature)
  const fallback: 'verified' | 'unsigned' = hasListSignature
    ? 'verified'
    : 'unsigned'
  try {
    await editableSvc.verifyFolderMemberList(payload)
    for (const member of payload.members) {
      statuses[member.user_id] = fallback
    }
  } catch (err) {
    if (err instanceof editableSvc.FolderMemberListInvalid && err.userId) {
      for (const member of payload.members) {
        statuses[member.user_id] =
          member.user_id === err.userId ? 'failed' : fallback
      }
    } else {
      for (const member of payload.members) {
        statuses[member.user_id] = hasListSignature ? 'failed' : 'unsigned'
      }
    }
  }
  signatureStatus.value = statuses
}

const members = computed<FolderMember[]>(() => response.value?.members ?? [])
const ownerId = computed(() => response.value?.folder_owner_id ?? null)
const isOwner = computed(
  () => ownerId.value !== null && ownerId.value === props.authenticatedUserId
)

/**
 * Mirrors the server's `can_reshare` gate (entity::permission). Anyone
 * who can reshare can also change role or revoke — that includes the
 * folder owner and any current Co-owner. Readers and Editors get the
 * read-only roster view from `79698bc` but no mutation affordances.
 */
const canReshare = computed(() => {
  if (isOwner.value) return true
  const self = members.value.find((m) => m.user_id === props.authenticatedUserId)
  return self?.share_role === 'co-owner'
})

function roleBadgeClass(role: ShareRole): string {
  switch (role) {
    case 'reader':
      return 'bg-brownish-200 text-brownish-900 dark:bg-brownish-700 dark:text-dirty-white'
    case 'editor':
      return 'bg-blueish-200 text-blueish-900 dark:bg-blueish-800 dark:text-blueish-100'
    case 'co-owner':
      return 'bg-redish-200 text-redish-900 dark:bg-redish-800 dark:text-redish-100'
  }
}

/**
 * On a folder, the role itself encodes whether the recipient can add
 * new files: Editor/Co-owner can upload, Reader cannot. Surface that
 * in a tooltip on the role badge so a hover answers the obvious
 * "can they upload?" question without growing the row.
 */
function roleBadgeTitle(role: ShareRole): string {
  switch (role) {
    case 'reader':
      return 'Reader · view only, cannot add new files'
    case 'editor':
      return 'Editor · can view, edit, and add new files'
    case 'co-owner':
      return 'Co-owner · full access, can re-share and add new files'
  }
}

function chunkedFingerprint(fp: string): string {
  return shareCrypto.formatFingerprint(fp)
}

/**
 * RSA-2048 fingerprints render as 16 hex-pair chunks (`XXXX-XXXX-…`)
 * which wraps to two lines at modal width. Single-line `XXXX-XXXX-…-XXXX`
 * keeps each member row at one line; the full value goes into the
 * `title=` tooltip for the curious reader.
 */
function abbreviatedFingerprint(fp: string): string {
  const full = shareCrypto.formatFingerprint(fp)
  if (full.length <= 19) return full
  return `${full.slice(0, 10)}…-${full.slice(-4)}`
}

function addedByLabel(member: FolderMember): string | null {
  if (member.is_owner) return null
  if (!member.signed_by_user_id) return 'Added by unknown'
  if (member.signed_by_user_id === ownerId.value) return 'Added by owner'
  return 'Added by Co-owner'
}

/**
 * Number of grants the candidate Co-owner has issued under this folder.
 * Cascade-revoke drops all of them when the Co-owner's own row is
 * removed. We surface the count in the confirmation prompt so the owner
 * sees the blast radius.
 */
function cascadeImpactFor(member: FolderMember): number {
  if (member.share_role !== 'co-owner') return 0
  return props.outgoingShares.filter(
    (share) => share.shared_by_user_id === member.user_id
  ).length
}

async function performRevoke(member: FolderMember): Promise<void> {
  if (!props.keypair.input) {
    errorNotification('Cannot revoke without an active session.')
    return
  }
  if (!response.value) {
    errorNotification('Member list not loaded; refresh and try again.')
    return
  }
  try {
    const timestamp = Math.floor(Date.now() / 1000)
    const signature = await shareCrypto.signAuditEvent(
      shareCrypto.buildAuditEventSigInput({
        senderId: props.authenticatedUserId,
        recipientId: member.user_id,
        fileId: props.folder.id,
        action: 'revoke',
        shareRoleBefore: member.share_role,
        shareRoleAfter: null,
        timestamp: BigInt(timestamp)
      }),
      props.keypair.input
    )
    const listSig = await buildPostRevokeListSignature(member, timestamp)
    await sharesApi.revokeShare(props.folder.id, member.user_id, {
      event_signature: signature,
      timestamp,
      members_list_signature: listSig
    })
    notification(
      'Share revoked',
      `${member.email ?? member.user_id} can no longer access this folder.`,
      'success'
    )
    emit('changed')
    await refresh()
  } catch (err) {
    errorNotification(err)
  }
}

/**
 * Compute and sign the `FolderMemberListV1` for the roster left behind
 * after revoking `revoked`. When the revoked member is a Co-owner, the
 * server cascade-drops every grant they issued under this folder — we
 * mirror that here so the bytes the client signs match the server's
 * post-revoke reconstruction.
 */
async function buildPostRevokeListSignature(
  revoked: FolderMember,
  signedAt: number
) {
  if (!response.value || !props.keypair.input) {
    throw new Error('Cannot sign list without loaded response and keypair')
  }
  const cascadeIds = new Set<string>([revoked.user_id])
  if (revoked.share_role === 'co-owner') {
    for (const m of response.value.members) {
      if (m.signed_by_user_id === revoked.user_id) {
        cascadeIds.add(m.user_id)
      }
    }
  }
  const remaining = response.value.members.filter((m) => !cascadeIds.has(m.user_id))
  const listInput = shareCrypto.buildFolderMemberListInput({
    folderId: props.folder.id,
    folderOwnerId: response.value.folder_owner_id,
    members: remaining.map((m) => ({
      userId: m.user_id,
      pubkeyFingerprintHex: m.pubkey_fingerprint,
      shareRole: m.share_role,
      isOwner: m.is_owner,
      signedByUserId: m.signed_by_user_id ?? response.value!.folder_owner_id
    })),
    membersSignedAt: BigInt(signedAt)
  })
  const { signature } = await shareCrypto.signFolderMemberList(
    listInput,
    props.keypair.input
  )
  return {
    signature,
    signed_at: signedAt,
    signed_by_user_id: props.authenticatedUserId
  }
}

function onRevokeClick(member: FolderMember): void {
  if (member.is_owner) return
  revokeConfirm.value = { member, affectedCount: cascadeImpactFor(member) }
}

function onChangeClick(member: FolderMember): void {
  if (member.is_owner || !member.email) return
  emit('change-role', { email: member.email, role: member.share_role })
}

async function confirmRevoke(): Promise<void> {
  if (!revokeConfirm.value) return
  const member = revokeConfirm.value.member
  revokeConfirm.value = null
  await performRevoke(member)
}

function cancelRevoke(): void {
  revokeConfirm.value = null
}
</script>

<template>
  <div data-testid="folder-members-view">
    <div class="flex items-center justify-between gap-3 mb-2">
      <span class="text-xs uppercase tracking-wider text-brownish-300 min-w-0 truncate">
        Members ({{ members.length }})
      </span>
      <BaseButton
        class="shrink-0"
        color="dark"
        small
        outline
        label="Refresh"
        :disabled="loading"
        data-testid="folder-members-view-refresh"
        @click.prevent="refresh"
      />
    </div>

    <p v-if="loading" class="text-xs text-brownish-300" data-testid="folder-members-view-loading">
      Loading members…
    </p>
    <p
      v-else-if="loadError"
      class="text-sm text-redish-700 dark:text-redish-300"
      data-testid="folder-members-view-error"
    >
      {{ loadError }}
    </p>

    <ul
      v-if="members.length > 0"
      class="space-y-1.5"
      data-testid="folder-members-view-list"
    >
      <li
        v-for="member in members"
        :key="member.user_id"
        class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 bg-brownish-50 dark:bg-brownish-800/60 rounded-lg px-3 py-2 text-sm"
        :data-testid="`folder-members-view-row-${member.user_id}`"
      >
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5 flex-wrap min-w-0">
            <span
              v-if="member.email"
              class="font-medium truncate min-w-0"
              :data-testid="`folder-members-view-row-${member.user_id}-email`"
            >
              {{ member.email }}
            </span>
            <span
              v-else
              class="font-medium text-brownish-300 truncate min-w-0 font-mono"
              :data-testid="`folder-members-view-row-${member.user_id}-id`"
            >
              {{ member.user_id }}
            </span>
            <span
              v-if="!member.is_owner"
              class="text-[11px] uppercase tracking-wider px-2 py-0.5 rounded-full"
              :class="roleBadgeClass(member.share_role)"
              :title="roleBadgeTitle(member.share_role)"
              :data-testid="`folder-members-view-row-${member.user_id}-role`"
            >
              {{ member.share_role }}
            </span>
            <span
              v-if="member.share_role !== 'reader' && !member.is_owner"
              class="text-[11px] text-brownish-500 dark:text-brownish-300 shrink-0"
              :title="`Can add new files to ${folder.name}`"
              :data-testid="`folder-members-view-row-${member.user_id}-can-upload`"
            >
              +upload
            </span>
            <span
              v-if="member.is_owner"
              class="text-[11px] uppercase tracking-wider px-2 py-0.5 rounded-full bg-greeny-200 text-greeny-900 dark:bg-greeny-800 dark:text-greeny-100"
              :data-testid="`folder-members-view-row-${member.user_id}-owner-badge`"
            >
              owner
            </span>
          </div>
          <div class="flex items-center gap-2 text-xs text-brownish-300 mt-0.5">
            <span
              class="font-mono truncate"
              :title="chunkedFingerprint(member.pubkey_fingerprint)"
              :data-testid="`folder-members-view-row-${member.user_id}-fingerprint`"
            >
              {{ abbreviatedFingerprint(member.pubkey_fingerprint) }}
            </span>
            <span
              v-if="addedByLabel(member)"
              class="truncate"
              :data-testid="`folder-members-view-row-${member.user_id}-added-by`"
            >
              · {{ addedByLabel(member) }}
            </span>
            <template v-if="!member.is_owner">
              <span
                v-if="signatureStatus[member.user_id] === 'verified'"
                class="inline-flex items-center text-greeny-600 dark:text-greeny-300 shrink-0"
                :data-testid="`folder-members-view-row-${member.user_id}-sig-verified`"
                title="Signature verified"
              >
                <BaseIcon :path="mdiCheckCircle" :size="12" />
              </span>
              <span
                v-else-if="signatureStatus[member.user_id] === 'failed'"
                class="inline-flex items-center text-redish-600 dark:text-redish-200 shrink-0"
                :data-testid="`folder-members-view-row-${member.user_id}-sig-failed`"
                title="Signature did not verify"
              >
                <BaseIcon :path="mdiAlertCircle" :size="12" />
              </span>
              <span
                v-else
                class="inline-flex items-center text-brownish-400 shrink-0"
                :data-testid="`folder-members-view-row-${member.user_id}-sig-unsigned`"
                title="Legacy row (no signature)"
              >
                <BaseIcon :path="mdiHelpCircleOutline" :size="12" />
              </span>
            </template>
          </div>
        </div>
        <div
          v-if="!member.is_owner && canReshare && member.user_id !== props.authenticatedUserId"
          class="flex items-center justify-end gap-1.5 shrink-0"
        >
          <BaseButton
            :title="member.email ? 'Change role' : 'Email unknown — cannot change role'"
            label="Change"
            color="dark"
            small
            :disabled="!member.email"
            :data-testid="`folder-members-view-row-${member.user_id}-change`"
            @click.prevent="onChangeClick(member)"
          />
          <BaseButton
            :icon="mdiTrashCan"
            color="danger"
            small
            rounded-full
            title="Revoke access"
            :data-testid="`folder-members-view-row-${member.user_id}-revoke`"
            @click.prevent="onRevokeClick(member)"
          />
        </div>
      </li>
    </ul>

    <RevokeConfirmModal
      v-if="revokeConfirm"
      :model-value="true"
      :recipient-label="revokeConfirm.member.email ?? revokeConfirm.member.user_id"
      :item-label="folder.name"
      :cascade-count="revokeConfirm.affectedCount"
      @update:model-value="(v) => { if (!v) cancelRevoke() }"
      @confirm="confirmRevoke"
      @cancel="cancelRevoke"
    />
  </div>
</template>
