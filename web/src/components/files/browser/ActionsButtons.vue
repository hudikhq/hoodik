<script setup lang="ts">
import { isPreviewable } from '!/preview'
import PureButton from '@/components/ui/PureButton.vue'
import {
  mdiTrashCan,
  mdiEye,
  mdiDownload,
  mdiPencil,
  mdiShareVariantOutline,
  mdiContentSave,
  mdiAccountArrowLeft
} from '@mdi/js'
import { useCapability } from '@/composables/useCapability'
import { SHARED_WITH_ME_DIR_ID } from '!/storage'
import type { AppFile } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: AppFile
  hideDelete?: boolean
  share?: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
  (event: 'details', file: AppFile): void
  (event: 'remove', file: AppFile): void
  (event: 'sharing', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'download', file: AppFile): void
  (event: 'fork', file: AppFile): void
  (event: 'leave', file: AppFile): void
}>()

const { sharingEnabled, forkEnabled } = useCapability()

const file = computed(() => props.modelValue)

/**
 * The synthetic "Shared with me" root has no row on disk — its id is a
 * client-only marker that would 400 every storage / shares endpoint it
 * reaches. Every action is rendered against `file`, so a single gate at
 * the top of the panel keeps the row stripped down to "you can't do
 * anything with this directly". Navigation into the folder still works
 * through the row's click handler in `TableFileRow`.
 */
const isSyntheticRoot = computed(() => file.value?.id === SHARED_WITH_ME_DIR_ID)

const hasPreview = computed(() => {
  if (isSyntheticRoot.value) return false
  return isPreviewable(file.value)
})

const hasDownload = computed(() => {
  if (isSyntheticRoot.value) return false
  return file.value?.mime !== 'dir' && file.value?.finished_upload_at
})

const canSharing = computed(() => {
  // Sharing a file shares its roster — every member of the share gets the
  // affordance so they can inspect who else is on it. The modal disables
  // mutation for callers without `can_reshare`.
  //
  // Incoming shares (`is_owner === false`) are by definition already
  // uploaded on the owner's side; the `finished_upload_at` gate only
  // suppresses the entry on owned rows mid-upload so half-finished files
  // can't be shared by accident.
  if (!sharingEnabled.value) return false
  if (!file.value) return false
  if (isSyntheticRoot.value) return false
  if (file.value.is_owner === false) return true
  return file.value.mime === 'dir' || !!file.value.finished_upload_at
})

/**
 * Fork is the Co-owner-only path that downloads, re-encrypts, and
 * uploads a fresh copy under the caller's key — used to detach the
 * file from the original share so revocation doesn't take it away.
 * Hidden on owned content (already mine) and on Reader / Editor rows
 * where the role can't authorise a re-encrypt against the caller's key
 * chain.
 */
const canFork = computed(() => {
  if (!sharingEnabled.value || !forkEnabled.value) return false
  if (!file.value) return false
  if (isSyntheticRoot.value) return false
  if (file.value.is_owner !== false) return false
  if (file.value.share_role !== 'co-owner') return false
  return file.value.mime !== 'dir'
})

/**
 * Drops only the caller's `user_files` row for a shared file. The Delete
 * affordance stays reserved for the owner action (permanent delete for
 * everyone), so recipients see a distinct Remove entry instead. The
 * follow-up confirm modal carries the disambiguating context ("X still
 * owns this file"), so the dropdown label can stay short.
 */
const canLeave = computed(() => {
  if (isSyntheticRoot.value) return false
  return !!(file.value && file.value.is_owner === false)
})

// Details + Rename + Delete all interpolate `file.id` into a real storage
// endpoint, so the synthetic root id must filter out of those too.
const canDetails = computed(() => !isSyntheticRoot.value)
const canRename = computed(() => {
  if (isSyntheticRoot.value) return false
  return !!(file.value?.is_owner && (file.value.finished_upload_at || file.value.mime === 'dir'))
})
const canDelete = computed(() => {
  if (props.hideDelete) return false
  if (isSyntheticRoot.value) return false
  return !!(file.value && file.value.is_owner !== false)
})
</script>

<template>
  <PureButton
    v-if="canDetails"
    :icon="mdiEye"
    @click="emits('details', file)"
    label="Details"
    name="details"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="hasPreview"
    :icon="mdiEye"
    :to="{
      name: 'file-preview',
      params: { id: file.id }
    }"
    label="Preview"
    name="preview"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    :icon="mdiDownload"
    @click="emits('download', file)"
    v-if="hasDownload"
    label="Download"
    name="download"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="canSharing"
    :icon="mdiShareVariantOutline"
    @click="emits('sharing', file)"
    label="Sharing"
    name="sharing"
    data-testid="actions-share-account"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="canFork"
    :icon="mdiContentSave"
    @click="emits('fork', file)"
    label="Fork"
    title="Save an independent copy to your drive — survives revocation"
    name="fork"
    data-testid="actions-fork"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="canLeave"
    :icon="mdiAccountArrowLeft"
    @click="emits('leave', file)"
    label="Remove"
    name="leave"
    data-testid="actions-leave"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="canDelete"
    :icon="mdiTrashCan"
    @click="emits('remove', file)"
    label="Delete"
    name="delete"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="canRename"
    :icon="mdiPencil"
    @click="emits('rename', file)"
    label="Rename"
    name="rename"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />
</template>
