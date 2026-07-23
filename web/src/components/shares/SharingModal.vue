<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  mdiAccountMultipleOutline,
  mdiClose,
  mdiFolderOutline,
  mdiFileOutline,
  mdiLink,
  mdiTrashCan
} from '@mdi/js'

import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import SharingPeopleAdd from '@/components/shares/SharingPeopleAdd.vue'
import SharingLinkPanel from '@/components/links/SharingLinkPanel.vue'
import FolderMembersView from '@/components/shares/FolderMembersView.vue'

import {
  crypto as shareCrypto,
  grantsStore,
  store as sharesStoreFactory
} from '!/shares'
import type { UserGrant } from '!/shares'
import { errorNotification, notification } from '!/index'

import type {
  AppFile,
  AppShare,
  FilesStore,
  KeyPair,
  LinksStore,
  ShareRole
} from 'types'

type TabKey = 'people' | 'link'

interface PrefillState {
  email: string
  role: ShareRole
  /** Folder add-files permission for the recipient — meaningless on a
   *  file share, but cheaper to carry uniformly than to branch the prop. */
  addFiles: boolean
}

const props = defineProps<{
  /** Single source of truth: file/folder the modal opens for. The modal
   *  surfaces even on Shared-with-me content; whether the caller can act
   *  is decided per-tab via `canWrite`. */
  file: AppFile | null
  authenticatedUserId: string
  keypair: KeyPair
  storage: FilesStore
  links: LinksStore
  /** Tab to land on when the modal opens. People is the default — the
   *  callers that want to drop the user straight onto the Link tab
   *  (the former "Public link" row action) pass 'link'. */
  initialTab?: TabKey
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const grants = grantsStore()
const sharesState = sharesStoreFactory()

const open = computed(() => props.file !== null)
const fileId = computed(() => props.file?.id ?? null)
const tab = ref<TabKey>(props.initialTab ?? 'people')

/**
 * Folder shares route through the signed members-list path, so every
 * folder caller — owner, Co-owner, Editor, Reader — gets `FolderMembersView`
 * in the People tab. The view shows the signed roster and gates revoke
 * affordances internally on the caller's own ownership of the folder.
 */
const isFolder = computed(() => props.file?.mime === 'dir')

/**
 * Public links are owner-only — only the file owner can mint, see, or
 * revoke them. Recipients cannot decrypt the link key (it's wrapped in
 * the owner's pubkey only), so surfacing the tab on the recipient side
 * just shows an empty placeholder. Hide the tab entirely when the caller
 * doesn't own the file; folders never expose the tab regardless.
 */
const isOwner = computed(() => props.file?.is_owner === true)
const showLinkTab = computed(() => !isFolder.value && isOwner.value)

const folderRecipients = computed<AppShare[]>(() => {
  if (!props.file) return []
  return sharesState.outgoingByFile[props.file.id] ?? []
})

async function onFolderMembersChanged(): Promise<void> {
  if (!props.file) return
  try {
    await grants.loadGrants(props.file.id, props.keypair)
  } catch (err) {
    errorNotification(err)
  }
}

function onFolderMemberChangeRole(payload: {
  email: string
  role: ShareRole
}): void {
  if (!canWrite.value) return
  // Folder add-files permission is encoded in the role:
  // Editor/Co-owner can upload, Reader cannot. Carry the implied add-files
  // value alongside the role so the form lands on the recipient's current
  // state rather than the default toggle.
  prefill.value = {
    email: payload.email,
    role: payload.role,
    addFiles: payload.role !== 'reader'
  }
}

/**
 * Owner has every right. A Co-owner has reshare + link rights on a file
 * they don't own. Anything else (Editor, Reader) lands
 * in the modal read-only — the surface stays available so the user can
 * inspect the existing grants, but every action is disabled.
 */
const canWrite = computed(() => {
  const f = props.file
  if (!f) return false
  if (f.is_owner) return true
  return f.share_role === 'co-owner'
})

/**
 * Co-owners can reshare but cannot grant Co-owner themselves — only the
 * file's owner can mint another Co-owner. Surface that
 * to `SharingPeopleAdd` so the role option drops out and the discover
 * step rejects the original owner as a recipient.
 */
const isCoOwnerReshare = computed(() => {
  const f = props.file
  if (!f) return false
  return f.is_owner === false && f.share_role === 'co-owner'
})

const fileOwnerId = computed<string | undefined>(() => {
  const f = props.file
  if (!f || f.is_owner) return undefined
  return f.user_id || undefined
})

const callerRoleLabel = computed(() => {
  const f = props.file
  if (!f) return ''
  if (f.is_owner) return 'Owner'
  switch (f.share_role) {
    case 'co-owner': return 'Co-owner'
    case 'editor': return 'Editor'
    case 'reader': return 'Reader'
    default: return ''
  }
})

const readOnlyExplanation = computed(() => {
  if (canWrite.value) return ''
  if (!props.file) return ''
  if (props.file.share_role === 'editor') {
    return 'Editors can view and edit, but only the owner or a Co-owner can change sharing.'
  }
  if (props.file.share_role === 'reader') {
    return 'Readers cannot change sharing on this file.'
  }
  return 'You do not have rights to change sharing on this file.'
})

const titleIcon = computed(() => {
  if (!props.file) return mdiFileOutline
  return props.file.mime === 'dir' ? mdiFolderOutline : mdiFileOutline
})

const grantList = computed(() =>
  fileId.value ? grants.grants(fileId.value).value : []
)

const userGrants = computed(() =>
  grantList.value.filter((g): g is UserGrant => g.kind === 'user')
)

const linkCount = computed(() =>
  grantList.value.filter((g) => g.kind === 'link').length
)

const loadingGrants = computed(() =>
  fileId.value ? grants.isLoading(fileId.value) : false
)

const loadError = computed(() =>
  fileId.value ? grants.errorOf(fileId.value) : null
)

const prefill = ref<PrefillState | null>(null)
const prefillEmail = computed(() => prefill.value?.email)
const prefillRole = computed(() => prefill.value?.role)
const prefillAddFiles = computed(() => prefill.value?.addFiles)

/**
 * Folder public links are intentionally out of scope on v1, and the
 * recipient-side view never shows the Link tab. Fall back to People when
 * the requested initial tab no longer exists in the DOM so the modal
 * doesn't land on an empty surface.
 */
function resolveInitialTab(): TabKey {
  if (props.initialTab === 'link' && !showLinkTab.value) return 'people'
  return props.initialTab ?? 'people'
}

watch(
  () => props.file,
  async (next, previous) => {
    if (!next) return
    if (previous && previous.id === next.id) return
    tab.value = resolveInitialTab()
    prefill.value = null
    try {
      await grants.loadGrants(next.id, props.keypair, showLinkTab.value)
    } catch (err) {
      errorNotification(err)
    }
  },
  { immediate: true }
)

watch(
  () => props.initialTab,
  (value) => {
    if (!value) return
    if (value === 'link' && !showLinkTab.value) return
    tab.value = value
  }
)

function close(): void {
  emit('close')
}

function fingerprintLabel(grant: UserGrant): string {
  return shareCrypto.formatFingerprint(grant.user.recipient_pubkey_fingerprint)
}

function abbreviatedFingerprint(grant: UserGrant): string {
  const full = fingerprintLabel(grant)
  if (full.length <= 19) return full
  return `${full.slice(0, 10)}…-${full.slice(-4)}`
}

function roleLabel(role: ShareRole): string {
  switch (role) {
    case 'reader': return 'Reader'
    case 'editor': return 'Editor'
    case 'co-owner': return 'Co-owner'
    default: return role
  }
}

/**
 * The role select is informational on v1 — changing the role is the
 * same idempotent POST as a fresh share, so we
 * prefill the add-recipient form with the recipient's email and current
 * role so the user can re-submit at the desired settings. Add-files is
 * computed from the role on file shares (no folder == no toggle).
 */
function changeRole(grant: UserGrant): void {
  if (!canWrite.value) return
  prefill.value = {
    email: grant.user.recipient_email,
    role: grant.user.share_role,
    addFiles: grant.user.share_role !== 'reader'
  }
}

async function revokeUser(grant: UserGrant): Promise<void> {
  if (!canWrite.value || !props.keypair.input || !props.file) return
  try {
    const timestamp = Math.floor(Date.now() / 1000)
    const signature = await shareCrypto.signAuditEvent(
      shareCrypto.buildAuditEventSigInput({
        senderId: props.authenticatedUserId,
        recipientId: grant.user.recipient_id,
        fileId: props.file.id,
        action: 'revoke',
        shareRoleBefore: grant.user.share_role,
        shareRoleAfter: null,
        timestamp: BigInt(timestamp)
      }),
      props.keypair.input
    )
    await grants.revokeGrant(props.file.id, grant.user.recipient_id, {
      event_signature: signature,
      timestamp
    })
    notification(
      'Share revoked',
      `${grant.user.recipient_email} can no longer access ${props.file.name}.`,
      'success'
    )
  } catch (err) {
    errorNotification(err)
  }
}

function onPeopleOk(): void {
  // Close the modal after a successful share. The recipients list lives
  // on the same surface — the caller reopens to inspect or to add
  // another recipient. Staying open after a submit would block the
  // pointer events under the overlay, which trips up logout / nav.
  prefill.value = null
  close()
}

function onPeopleCancel(): void {
  prefill.value = null
  close()
}
</script>

<template>
  <CardBoxModal
    v-if="open"
    :model-value="open"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="close"
  >
    <div class="sticky top-0 z-10 -mx-4 px-4 -mt-4 pt-4 pb-2 bg-white dark:bg-brownish-900 border-b border-brownish-200 dark:border-brownish-700">
      <div class="flex items-start justify-between gap-3 mb-3">
        <div class="flex items-start gap-2 min-w-0">
          <BaseIcon :path="titleIcon" :size="24" class="shrink-0 mt-0.5 text-brownish-300 dark:text-brownish-200" />
          <div class="min-w-0">
            <h2
              class="text-lg sm:text-xl font-semibold leading-tight truncate"
              :title="props.file?.name ?? 'Sharing'"
            >
              {{ props.file?.name ?? 'Sharing' }}
            </h2>
            <span
              v-if="callerRoleLabel"
              class="inline-block mt-1 text-[11px] uppercase tracking-wider px-2 py-0.5 rounded-full bg-brownish-100 dark:bg-brownish-800 text-brownish-700 dark:text-brownish-200"
              data-testid="sharing-modal-role-badge"
            >
              {{ callerRoleLabel }}
            </span>
          </div>
        </div>
        <button
          type="button"
          class="shrink-0 -mt-1 -mr-1 w-11 h-11 inline-flex items-center justify-center rounded-full text-brownish-400 hover:text-brownish-100 hover:bg-brownish-100 dark:hover:bg-brownish-800 transition-colors"
          title="Close"
          data-testid="sharing-modal-close"
          @click.prevent="close"
        >
          <BaseIcon :path="mdiClose" :size="20" />
        </button>
      </div>

      <div
        v-if="!canWrite"
        class="mb-2 px-3 py-2 rounded-lg text-xs sm:text-sm bg-brownish-100 dark:bg-brownish-800 text-brownish-700 dark:text-brownish-200"
        data-testid="sharing-modal-readonly-banner"
      >
        {{ readOnlyExplanation }}
      </div>

      <div v-if="showLinkTab" class="flex -mb-2">
        <button
          type="button"
          class="flex-1 sm:flex-initial min-h-11 px-3 sm:px-4 py-2 text-sm inline-flex items-center justify-center gap-2 border-b-2 transition-colors"
          :class="tab === 'people'
            ? 'border-redish-500 text-redish-500 dark:text-redish-200 font-medium'
            : 'border-transparent text-brownish-400 dark:text-brownish-300 hover:text-brownish-100'"
          data-testid="sharing-modal-tab-people"
          @click.prevent="tab = 'people'"
        >
          <BaseIcon :path="mdiAccountMultipleOutline" :size="16" />
          <span>People</span>
          <span class="text-xs opacity-70">{{ userGrants.length }}</span>
        </button>
        <button
          type="button"
          class="flex-1 sm:flex-initial min-h-11 px-3 sm:px-4 py-2 text-sm inline-flex items-center justify-center gap-2 border-b-2 transition-colors"
          :class="tab === 'link'
            ? 'border-redish-500 text-redish-500 dark:text-redish-200 font-medium'
            : 'border-transparent text-brownish-400 dark:text-brownish-300 hover:text-brownish-100'"
          data-testid="sharing-modal-tab-link"
          @click.prevent="tab = 'link'"
        >
          <BaseIcon :path="mdiLink" :size="16" />
          <span>Public link</span>
          <span class="text-xs opacity-70">{{ linkCount }}</span>
        </button>
      </div>
    </div>

    <p
      v-if="loadError"
      class="mt-3 text-sm text-redish-700 dark:text-redish-300"
      data-testid="sharing-modal-load-error"
    >
      {{ loadError }}
    </p>

    <div v-if="tab === 'people'" class="pt-6 space-y-4">
      <FolderMembersView
        v-if="isFolder && props.file"
        :folder="props.file"
        :authenticated-user-id="props.authenticatedUserId"
        :keypair="props.keypair"
        :outgoing-shares="folderRecipients"
        data-testid="sharing-modal-folder-members"
        @changed="onFolderMembersChanged"
        @change-role="onFolderMemberChangeRole"
      />
      <div
        v-if="isFolder && props.file"
        class="border-t border-brownish-200 dark:border-brownish-700 pt-4"
      >
        <SharingPeopleAdd
          :file="props.file"
          :authenticated-user-id="props.authenticatedUserId"
          :keypair="props.keypair"
          :prefill-email="prefillEmail"
          :prefill-role="prefillRole"
          :prefill-add-files="prefillAddFiles"
          :read-only="!canWrite"
          :disable-co-owner="isCoOwnerReshare"
          :owner-id="fileOwnerId"
          @ok="onPeopleOk"
          @cancel="onPeopleCancel"
        />
      </div>
      <template v-else>
        <div data-testid="sharing-modal-people-list">
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs uppercase tracking-wider text-brownish-300">
              Recipients ({{ userGrants.length }})
            </span>
          </div>
          <p
            v-if="loadingGrants && userGrants.length === 0"
            class="text-xs text-brownish-300"
          >
            Loading…
          </p>
          <p
            v-else-if="userGrants.length === 0"
            class="text-sm text-brownish-300 px-3 py-3 rounded-lg bg-brownish-50 dark:bg-brownish-800/40"
            data-testid="sharing-modal-people-empty"
          >
            No accounts have access yet.
          </p>
          <ul v-else class="space-y-1.5">
            <li
              v-for="grant in userGrants"
              :key="grant.recipient_id"
              class="px-3 py-2 rounded-lg bg-brownish-50 dark:bg-brownish-800/60 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2"
              :data-testid="`sharing-modal-people-row-${grant.recipient_id}`"
            >
              <div class="min-w-0 sm:flex-1">
                <div class="text-sm font-medium truncate">{{ grant.user.recipient_email }}</div>
                <div
                  class="text-xs font-mono text-brownish-400 truncate"
                  :title="fingerprintLabel(grant)"
                >
                  {{ abbreviatedFingerprint(grant) }}
                </div>
              </div>
              <div class="flex items-center justify-end gap-1.5 shrink-0">
                <span
                  class="text-[11px] uppercase tracking-wider px-2 py-0.5 rounded-full bg-brownish-200 dark:bg-brownish-700 text-brownish-700 dark:text-brownish-100"
                  :data-testid="`sharing-modal-role-badge-${grant.recipient_id}`"
                >
                  {{ roleLabel(grant.user.share_role) }}
                </span>
                <BaseButton
                  v-if="canWrite"
                  title="Change role"
                  label="Change"
                  color="dark"
                  small
                  :data-testid="`sharing-modal-change-role-${grant.recipient_id}`"
                  @click.prevent="changeRole(grant)"
                />
                <BaseButtonConfirm
                  title="Revoke access"
                  :icon="mdiTrashCan"
                  color="danger"
                  small
                  rounded-full
                  confirm-label="Revoke"
                  cancel-label="Cancel"
                  :disabled="!canWrite"
                  :data-testid="`sharing-modal-revoke-${grant.recipient_id}`"
                  @confirm="() => revokeUser(grant)"
                />
              </div>
            </li>
          </ul>
        </div>

        <div class="border-t border-brownish-200 dark:border-brownish-700 pt-4">
          <SharingPeopleAdd
            :file="props.file"
            :authenticated-user-id="props.authenticatedUserId"
            :keypair="props.keypair"
            :prefill-email="prefillEmail"
            :prefill-role="prefillRole"
            :prefill-add-files="prefillAddFiles"
            :read-only="!canWrite"
            :disable-co-owner="isCoOwnerReshare"
            :owner-id="fileOwnerId"
            @ok="onPeopleOk"
            @cancel="onPeopleCancel"
          />
        </div>
      </template>
    </div>

    <div v-else-if="tab === 'link'" class="pt-4">
      <SharingLinkPanel
        :source="props.file ?? undefined"
        :storage="props.storage"
        :links="props.links"
        :kp="props.keypair"
        :read-only="!canWrite"
      />
    </div>
  </CardBoxModal>
</template>
