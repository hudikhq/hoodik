<script setup lang="ts">
import { mdiAccountPlus } from '@mdi/js'

import BaseButton from '@/components/ui/BaseButton.vue'
import FingerprintMismatchModal from '@/components/shares/FingerprintMismatchModal.vue'
import SharingPeopleAddGroupPanel from '@/components/shares/SharingPeopleAddGroupPanel.vue'
import SharingPeopleAddRecipientPanel from '@/components/shares/SharingPeopleAddRecipientPanel.vue'
import { AppField } from '@/components/form'

import {
  useSharingPeopleAdd,
  type SharingPeopleAddProps
} from '@/components/shares/composables/useSharingPeopleAdd'
import type { AppShare } from 'types'

const props = defineProps<SharingPeopleAddProps>()

const emit = defineEmits<{
  (e: 'ok', shares: AppShare[]): void
  (e: 'cancel'): void
}>()

const {
  isDir,
  folderEditable,
  email,
  recipient,
  formattedFingerprint,
  role,
  discoverError,
  submitting,
  groupSuggestions,
  selectedGroup,
  progress,
  recentRecipients,
  mismatchPayload,
  discover,
  showTrustedPill,
  showUnknownPill,
  onMismatchAccept,
  onMismatchCancel,
  confirmDisabled,
  lastVerifiedLabel,
  submit,
  cancel,
  abortWalk,
  fillRecent,
  folderHint,
  showFolderHint,
  abbreviatedFormattedFingerprint,
  roleDescription,
  progressLabel,
  overlayStatus,
  determinate,
  indeterminateActive,
  capMessage
} = useSharingPeopleAdd(props, emit)
</script>

<template>
  <div class="space-y-3" data-testid="share-dialog-target">
    <div v-if="!file">
      <p class="text-sm">No file selected.</p>
    </div>
    <p
      v-else-if="showFolderHint"
      class="text-xs text-brownish-300"
      data-testid="share-dialog-folder-hint"
    >
      {{ folderHint }}
    </p>

    <div>
      <AppField
        name="recipient-email"
        label="Add a recipient"
        v-model="email"
        :disabled="submitting || readOnly"
        @confirm="discover"
        placeholder="someone@example.com"
      />
      <div class="flex justify-between items-center mt-2 flex-wrap gap-2">
        <BaseButton
          color="dark"
          small
          label="Find user"
          :icon="mdiAccountPlus"
          :disabled="submitting || readOnly || !email.trim()"
          data-testid="share-dialog-discover"
          @click.prevent="discover"
        />
        <div v-if="recentRecipients.length" class="text-xs text-brownish-300 flex flex-wrap items-center gap-x-2 gap-y-1 min-w-0">
          <span class="opacity-70 shrink-0">Recent:</span>
          <button
            v-for="value in recentRecipients.slice(0, 3)"
            :key="value"
            type="button"
            class="max-w-[10rem] truncate underline hover:text-brownish-100"
            :title="value"
            :disabled="submitting || readOnly"
            @click.prevent="fillRecent(value)"
          >
            {{ value }}
          </button>
        </div>
      </div>
      <ul
        v-if="groupSuggestions.length"
        class="mt-2 flex flex-wrap gap-1.5 text-xs"
        data-testid="share-dialog-group-suggestions"
      >
        <li v-for="group in groupSuggestions" :key="group.id">
          <button
            type="button"
            class="px-2.5 py-1 rounded-full bg-brownish-100 dark:bg-brownish-800 text-brownish-700 dark:text-brownish-200 hover:bg-brownish-200 dark:hover:bg-brownish-700 transition-colors"
            :data-testid="`share-dialog-group-suggestion-${group.id}`"
            :disabled="submitting || readOnly"
            @click.prevent="() => { email = group.name; void discover() }"
          >
            {{ group.name }}
            <span v-if="group.memberCount !== null" class="opacity-70">· {{ group.memberCount }}</span>
          </button>
        </li>
      </ul>
      <p
        v-if="discoverError"
        class="mt-2 text-sm text-redish-700 dark:text-redish-300"
        data-testid="share-dialog-discover-error"
      >
        {{ discoverError }}
      </p>
    </div>

    <SharingPeopleAddGroupPanel
      v-if="selectedGroup"
      :group="selectedGroup"
      v-model:role="role"
      :submitting="submitting"
      :read-only="readOnly"
      :disable-co-owner="disableCoOwner"
    />

    <SharingPeopleAddRecipientPanel
      v-if="recipient"
      :recipient="recipient"
      v-model:role="role"
      v-model:folder-editable="folderEditable"
      :abbreviated-formatted-fingerprint="abbreviatedFormattedFingerprint"
      :formatted-fingerprint="formattedFingerprint"
      :role-description="roleDescription"
      :last-verified-label="lastVerifiedLabel"
      :cap-message="capMessage"
      :progress-label="progressLabel"
      :progress="progress"
      :is-dir="isDir"
      :submitting="submitting"
      :read-only="readOnly"
      :disable-co-owner="disableCoOwner"
      :show-trusted-pill="showTrustedPill"
      :show-unknown-pill="showUnknownPill"
      :determinate="determinate"
      :indeterminate-active="indeterminateActive"
      @abort-walk="abortWalk"
    />

    <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2 pt-2">
      <BaseButton
        label="Cancel"
        color="info"
        outline
        :disabled="submitting"
        @click.prevent="cancel"
      />
      <BaseButton
        label="Share"
        color="info"
        :disabled="confirmDisabled"
        data-testid="share-dialog-submit"
        @click.prevent="submit"
      />
    </div>

    <transition
      enter-active-class="transition-opacity duration-150"
      leave-active-class="transition-opacity duration-150"
      enter-from-class="opacity-0"
      leave-to-class="opacity-0"
    >
      <div
        v-if="submitting"
        class="absolute inset-0 z-20 flex items-center justify-center rounded-lg bg-white/80 dark:bg-brownish-900/80 backdrop-blur-[2px]"
        data-testid="share-dialog-submit-overlay"
        @click.stop
      >
        <div class="flex flex-col items-center gap-3 px-6 py-4 text-center max-w-full">
          <span
            class="inline-block w-7 h-7 rounded-full border-2 border-brownish-200 border-t-redish-500 share-dialog-spinner"
            aria-hidden="true"
          />
          <p
            class="text-sm text-brownish-700 dark:text-brownish-100 max-w-xs truncate"
            data-testid="share-dialog-submit-overlay-status"
            :title="overlayStatus"
          >
            {{ overlayStatus }}
          </p>
          <p
            v-if="progressLabel"
            class="text-xs text-brownish-400 max-w-xs truncate"
            :title="progressLabel"
          >
            {{ progressLabel }}
          </p>
          <button
            v-if="(indeterminateActive || determinate) && progress.phase !== 'submitting'"
            type="button"
            class="text-xs underline text-redish-500 dark:text-redish-200 hover:text-redish-300"
            data-testid="share-dialog-submit-overlay-cancel"
            @click.prevent="abortWalk"
          >
            Cancel
          </button>
        </div>
      </div>
    </transition>
  </div>

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

<style scoped>
@keyframes share-dialog-spinner-keyframes {
  to { transform: rotate(360deg); }
}
.share-dialog-spinner {
  animation: share-dialog-spinner-keyframes 0.7s linear infinite;
}
</style>
