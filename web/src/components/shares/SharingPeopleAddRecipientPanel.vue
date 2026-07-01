<script setup lang="ts">
import { computed } from 'vue'
import { mdiCheckCircleOutline, mdiAlertCircleOutline, mdiShieldKeyOutline } from '@mdi/js'

import BaseIcon from '@/components/ui/BaseIcon.vue'
import SharingPeopleAddRoleChips from '@/components/shares/SharingPeopleAddRoleChips.vue'
import type { DiscoveredUser, ShareRole } from 'types'

interface ProgressView {
  current: number
  total: number
  phase: 'idle' | 'discovering' | 'walking' | 'preparing' | 'submitting' | 'done'
  walked: number
  capExceeded: boolean
}

const props = defineProps<{
  recipient: DiscoveredUser
  role: ShareRole
  folderEditable: boolean
  abbreviatedFormattedFingerprint: string
  formattedFingerprint: string
  roleDescription: string
  lastVerifiedLabel: string
  capMessage: string
  progressLabel: string
  progress: ProgressView
  isDir: boolean
  submitting: boolean
  readOnly?: boolean
  disableCoOwner?: boolean
  showTrustedPill: boolean
  showUnknownPill: boolean
  determinate: boolean
  indeterminateActive: boolean
}>()

const emit = defineEmits<{
  (e: 'update:role', value: ShareRole): void
  (e: 'update:folderEditable', value: boolean): void
  (e: 'abort-walk'): void
}>()

const role = computed({
  get: () => props.role,
  set: (value: ShareRole) => emit('update:role', value)
})

const folderEditable = computed({
  get: () => props.folderEditable,
  set: (value: boolean) => emit('update:folderEditable', value)
})
</script>

<template>
  <div
    class="border border-brownish-200 dark:border-brownish-700 rounded-lg p-3 space-y-3"
  >
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-1 sm:gap-2 min-w-0">
      <span class="text-sm font-medium truncate" data-testid="share-dialog-recipient-email">
        {{ recipient.email }}
      </span>
      <span
        class="text-xs font-mono text-brownish-400 truncate shrink-0"
        :title="formattedFingerprint"
        data-testid="share-dialog-fingerprint"
      >
        {{ abbreviatedFormattedFingerprint }}
      </span>
    </div>

    <div>
      <span class="block text-xs uppercase tracking-wider text-brownish-300 mb-1.5">Role</span>
      <SharingPeopleAddRoleChips
        v-model="role"
        testid-prefix="share-dialog-role"
        :disabled="submitting || readOnly"
        :disable-co-owner="disableCoOwner"
      />
      <p class="mt-2 text-xs text-brownish-300">
        {{ roleDescription }}
        <span
          v-if="disableCoOwner"
          class="block mt-0.5"
          data-testid="share-dialog-coowner-disabled-hint"
        >
          Only the file's owner can grant Co-owner.
        </span>
      </p>
    </div>

    <label
      v-if="isDir"
      class="flex gap-2 items-start text-sm cursor-pointer min-h-[1.75rem]"
      :class="{ 'opacity-50 cursor-not-allowed': role === 'reader' }"
      data-testid="share-dialog-folder-editable"
    >
      <input
        type="checkbox"
        v-model="folderEditable"
        :disabled="submitting || readOnly || role === 'reader'"
        class="mt-0.5"
        data-testid="share-dialog-folder-editable-toggle"
      />
      <span class="min-w-0">
        Allow them to add new files
        <span
          v-if="role === 'reader'"
          class="block text-xs text-brownish-300"
          data-testid="share-dialog-folder-editable-disabled-hint"
        >
          Pick Editor or Co-owner to enable
        </span>
      </span>
    </label>

    <div
      v-if="showTrustedPill"
      class="px-2.5 py-1.5 bg-greeny-100 dark:bg-greeny-900/30 text-greeny-900 dark:text-greeny-100 rounded-lg text-xs flex items-start gap-2"
      data-testid="share-dialog-trusted"
    >
      <BaseIcon :path="mdiCheckCircleOutline" :size="14" class="mt-0.5 shrink-0" />
      <span>{{ lastVerifiedLabel }} The fingerprint still matches.</span>
    </div>

    <div
      v-if="showUnknownPill"
      class="px-2.5 py-1.5 bg-brownish-100 dark:bg-brownish-800/60 text-brownish-700 dark:text-brownish-200 rounded-lg text-xs flex items-start gap-2"
      data-testid="share-dialog-unknown"
    >
      <BaseIcon :path="mdiShieldKeyOutline" :size="14" class="mt-0.5 shrink-0" />
      <span>
        First time sharing with this account. Compare the fingerprint out of band if you
        want to be certain — we'll warn loudly if it ever changes.
      </span>
    </div>

    <div
      v-if="progress.capExceeded"
      class="px-2.5 py-1.5 bg-redish-100 dark:bg-redish-900/40 text-redish-700 dark:text-redish-200 rounded-lg text-xs flex items-start gap-2"
      data-testid="share-dialog-cap-exceeded"
    >
      <BaseIcon :path="mdiAlertCircleOutline" :size="14" class="mt-0.5 shrink-0" />
      <span>{{ capMessage }}</span>
    </div>

    <div
      v-if="!submitting && progress.phase !== 'idle' && progress.phase !== 'done'"
      class="text-xs"
      data-testid="share-dialog-progress"
    >
      <div class="flex items-center justify-between gap-2">
        <span class="truncate">{{ progressLabel }}</span>
        <button
          v-if="indeterminateActive || determinate"
          type="button"
          class="shrink-0 text-redish-500 dark:text-redish-200 underline"
          data-testid="share-dialog-progress-cancel"
          @click.prevent="emit('abort-walk')"
        >
          Cancel
        </button>
      </div>
      <div
        v-if="determinate"
        class="w-full bg-brownish-200 dark:bg-brownish-700 h-1 rounded mt-1.5 overflow-hidden"
      >
        <div
          class="bg-redish-500 h-1 rounded transition-all"
          :style="{
            width: `${Math.min(100, (progress.current / Math.max(progress.total, 1)) * 100)}%`
          }"
        />
      </div>
      <div
        v-else-if="indeterminateActive"
        class="w-full bg-brownish-200 dark:bg-brownish-700 h-1 rounded mt-1.5 overflow-hidden"
      >
        <div class="bg-redish-500 h-1 w-1/3 rounded share-dialog-indeterminate" />
      </div>
    </div>
  </div>
</template>

<style scoped>
@keyframes share-dialog-indeterminate-keyframes {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(300%); }
}
.share-dialog-indeterminate {
  animation: share-dialog-indeterminate-keyframes 1.4s linear infinite;
}
</style>
