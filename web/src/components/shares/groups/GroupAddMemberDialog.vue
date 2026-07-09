<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { mdiCheckCircleOutline, mdiShieldKeyOutline, mdiAlertCircleOutline } from '@mdi/js'

import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import FingerprintMismatchModal from '@/components/shares/FingerprintMismatchModal.vue'
import { AppField } from '@/components/form'

import {
  api as sharesApi,
  crypto as shareCrypto,
  groups as shareGroups,
  trustedFingerprintsStore,
  DiscoverUserError
} from '!/shares'
import { errorNotification, notification } from '!/index'

import type { DiscoveredUser, GroupRole } from 'types'

const props = defineProps<{
  modelValue: boolean
  groupId: string
  groupName: string
  /** When true the caller may grant co-owner (group owner only). A
   *  co-owner manager can add reader/editor but never another co-owner —
   *  mirrors the server's privilege-escalation guard, surfaced fail-closed
   *  in the UI so the disallowed option never renders. */
  canGrantCoOwner?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'added', recipient: DiscoveredUser): void
  (e: 'cancel'): void
}>()

const trusted = trustedFingerprintsStore()

const open = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emit('update:modelValue', value)
})

const email = ref('')
const recipient = ref<DiscoveredUser | null>(null)
const fingerprintHex = ref('')
const formattedFingerprint = ref('')
const groupRole = ref<GroupRole>('reader')
const submitting = ref(false)
const errorText = ref<string | null>(null)
const trustStatus = ref<'unknown' | 'trusted-fresh' | 'trusted-stale' | 'mismatch'>('unknown')

const mismatchPayload = ref<{
  recipientEmail: string
  cachedFingerprint: string
  newFingerprint: string
  lastVerifiedAt: number
} | null>(null)

watch(
  () => props.modelValue,
  (value) => {
    if (value) {
      email.value = ''
      recipient.value = null
      fingerprintHex.value = ''
      formattedFingerprint.value = ''
      groupRole.value = 'reader'
      submitting.value = false
      errorText.value = null
      trustStatus.value = 'unknown'
      mismatchPayload.value = null
    }
  }
)

async function discover(): Promise<void> {
  const trimmed = email.value.trim()
  if (!trimmed) {
    errorText.value = 'Enter the member email first.'
    return
  }
  errorText.value = null
  trustStatus.value = 'unknown'
  try {
    const user = await sharesApi.discoverUser(trimmed)
    recipient.value = user
    fingerprintHex.value = shareCrypto.fingerprintForUser(user)
    formattedFingerprint.value = shareCrypto.formatFingerprint(fingerprintHex.value)
    const cached = trusted.lookup(user.user_id)
    if (cached) {
      if (cached.pubkeyFingerprint !== fingerprintHex.value) {
        trustStatus.value = 'mismatch'
        mismatchPayload.value = {
          recipientEmail: user.email,
          cachedFingerprint: shareCrypto.formatFingerprint(cached.pubkeyFingerprint),
          newFingerprint: formattedFingerprint.value,
          lastVerifiedAt: cached.lastVerifiedAt
        }
      } else if (trusted.isStale(user.user_id)) {
        trustStatus.value = 'trusted-stale'
      } else {
        trustStatus.value = 'trusted-fresh'
      }
    } else {
      trustStatus.value = 'unknown'
    }
  } catch (err) {
    recipient.value = null
    if (err instanceof DiscoverUserError) {
      switch (err.kind) {
        case 'not_found':
          errorText.value = "We couldn't find a Hoodik account for that email."
          break
        case 'self':
          errorText.value = 'That email is your own — you can\'t add yourself.'
          break
        case 'rate_limited':
          errorText.value = 'Too many lookups. Wait a minute, then try again.'
          break
        case 'feature_disabled':
          errorText.value = 'Sharing is currently disabled on this server.'
          break
        default:
          errorText.value = err.message
      }
    } else {
      errorText.value = (err as Error).message
    }
  }
}

async function submit(): Promise<void> {
  if (!recipient.value) return
  if (trustStatus.value === 'mismatch') return
  submitting.value = true
  errorText.value = null
  try {
    await shareGroups.addMember({
      groupId: props.groupId,
      recipient: recipient.value,
      groupRole: groupRole.value
    })
    trusted.trustFingerprint(recipient.value.user_id, fingerprintHex.value, 'silent')
    notification(
      'Member added',
      `${recipient.value.email} is now part of "${props.groupName}".`,
      'success'
    )
    emit('added', recipient.value)
    open.value = false
  } catch (err) {
    errorText.value = err instanceof Error ? err.message : 'Failed to add the member'
    errorNotification(err)
  } finally {
    submitting.value = false
  }
}

function cancel(): void {
  emit('cancel')
  open.value = false
}

function onMismatchAccept(): void {
  if (!recipient.value || !fingerprintHex.value) return
  trusted.trustFingerprint(recipient.value.user_id, fingerprintHex.value, 'in-person')
  trustStatus.value = 'trusted-fresh'
  mismatchPayload.value = null
}

function onMismatchCancel(): void {
  recipient.value = null
  fingerprintHex.value = ''
  formattedFingerprint.value = ''
  trustStatus.value = 'unknown'
  mismatchPayload.value = null
}

const submitDisabled = computed(
  () =>
    submitting.value || !recipient.value || trustStatus.value === 'mismatch'
)

const showTrustedPill = computed(
  () => trustStatus.value === 'trusted-fresh' && recipient.value !== null
)

const showUnknownPill = computed(
  () =>
    recipient.value !== null &&
    (trustStatus.value === 'unknown' || trustStatus.value === 'trusted-stale')
)
</script>

<template>
  <CardBoxModal
    v-if="open"
    :title="`Add member to ${groupName}`"
    :model-value="open"
    has-cancel
    hide-submit
    @update:model-value="(value) => (open = value)"
    @cancel="cancel"
  >
    <div class="space-y-4">
      <div>
        <AppField
          name="group-add-email"
          label="Member email"
          v-model="email"
          :disabled="submitting"
          placeholder="someone@example.com"
          @confirm="discover"
        />
        <div class="mt-2">
          <BaseButton
            color="dark"
            small
            label="Find user"
            :disabled="submitting || !email.trim()"
            data-testid="group-add-member-discover"
            @click.prevent="discover"
          />
        </div>
        <p
          v-if="errorText"
          class="mt-2 text-sm text-redish-700 dark:text-redish-300"
          data-testid="group-add-member-error"
        >
          {{ errorText }}
        </p>
      </div>

      <div
        v-if="recipient"
        class="border border-brownish-200 dark:border-brownish-700 rounded-lg p-3 space-y-3"
      >
        <div>
          <div class="text-xs uppercase tracking-wider text-brownish-300">Member</div>
          <div class="text-sm truncate" data-testid="group-add-member-email">
            {{ recipient.email }}
          </div>
        </div>
        <div>
          <div class="text-xs uppercase tracking-wider text-brownish-300">Public-key fingerprint</div>
          <div
            class="text-xs font-mono break-all text-brownish-700 dark:text-brownish-200"
            data-testid="group-add-member-fingerprint"
          >
            {{ formattedFingerprint }}
          </div>
        </div>
        <div class="space-y-1.5">
          <div class="text-xs uppercase tracking-wider text-brownish-300">
            Group role
          </div>
          <p class="text-xs text-brownish-300">
            What this member may do to the group. Sharing a file to the group
            reaches every member regardless of this role.
          </p>
          <label class="flex gap-2 items-center text-sm min-h-[2rem] cursor-pointer">
            <input
              type="radio"
              v-model="groupRole"
              value="reader"
              :disabled="submitting"
              data-testid="group-add-member-role-reader"
            />
            <span>Reader — receives shares; can't manage the group.</span>
          </label>
          <label class="flex gap-2 items-center text-sm min-h-[2rem] cursor-pointer">
            <input
              type="radio"
              v-model="groupRole"
              value="editor"
              :disabled="submitting"
              data-testid="group-add-member-role-editor"
            />
            <span>Editor — can share files into the group.</span>
          </label>
          <label
            v-if="canGrantCoOwner"
            class="flex gap-2 items-center text-sm min-h-[2rem] cursor-pointer"
          >
            <input
              type="radio"
              v-model="groupRole"
              value="co-owner"
              :disabled="submitting"
              data-testid="group-add-member-role-coowner"
            />
            <span>Co-owner — can also manage the roster.</span>
          </label>
        </div>
        <div
          v-if="showTrustedPill"
          class="px-3 py-2 bg-greeny-100 dark:bg-greeny-900/30 text-greeny-900 dark:text-greeny-100 rounded-lg text-xs flex items-start gap-2"
          data-testid="group-add-member-trusted"
        >
          <BaseIcon :path="mdiCheckCircleOutline" :size="14" class="mt-0.5 shrink-0" />
          <span>The fingerprint matches what we have on file for this member.</span>
        </div>
        <div
          v-if="showUnknownPill"
          class="px-3 py-2 bg-brownish-100 dark:bg-brownish-800/60 text-brownish-700 dark:text-brownish-200 rounded-lg text-xs flex items-start gap-2"
          data-testid="group-add-member-unknown"
        >
          <BaseIcon :path="mdiShieldKeyOutline" :size="14" class="mt-0.5 shrink-0" />
          <span>
            First time adding this member. Compare the fingerprint out of band if you want
            to be certain — we'll warn loudly if it ever changes.
          </span>
        </div>
        <div
          v-if="trustStatus === 'mismatch'"
          class="px-3 py-2 bg-redish-100 dark:bg-redish-900/40 text-redish-700 dark:text-redish-200 rounded-lg text-xs flex items-start gap-2"
          data-testid="group-add-member-mismatch"
        >
          <BaseIcon :path="mdiAlertCircleOutline" :size="14" class="mt-0.5 shrink-0" />
          <span>
            This member's key has changed since you last verified it. Resolve the mismatch before adding.
          </span>
        </div>
      </div>
    </div>

    <template #buttons>
      <BaseButton
        label="Add member"
        color="info"
        :disabled="submitDisabled"
        data-testid="group-add-member-submit"
        @click.prevent="submit"
      />
      <BaseButton label="Cancel" color="info" outline @click.prevent="cancel" />
    </template>
  </CardBoxModal>

  <FingerprintMismatchModal
    v-if="mismatchPayload"
    :model-value="true"
    :recipient-email="mismatchPayload.recipientEmail"
    :cached-fingerprint="mismatchPayload.cachedFingerprint"
    :new-fingerprint="mismatchPayload.newFingerprint"
    :last-verified-at="mismatchPayload.lastVerifiedAt"
    @accept="onMismatchAccept"
    @cancel="onMismatchCancel"
    @update:model-value="(v) => { if (!v) onMismatchCancel() }"
  />
</template>
