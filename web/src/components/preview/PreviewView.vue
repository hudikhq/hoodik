<script setup lang="ts">
import {
  mdiTrashCan,
  mdiDownload,
  mdiFileOutline,
  mdiClose,
  mdiInformationSlabCircleOutline,
  mdiArrowLeft,
  mdiArrowRight
} from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import PreviewImage from './PreviewImage.vue'
import { computed, onMounted, onUnmounted } from 'vue'
import type { Preview } from '!/preview'

const props = defineProps<{
  modelValue: Preview
  hidePreviousAndNext?: boolean
  hideDelete?: boolean
  hideClose?: boolean
}>()

const emits = defineEmits<{
  (event: 'download', file: Preview): void
  (event: 'remove', file: Preview): void
  (event: 'details', file: Preview): void
  (event: 'cancel', file: Preview): void
  (event: 'previous', id: string | undefined): void
  (event: 'next', id: string | undefined): void
}>()

const preview = computed(() => props.modelValue)
const previewType = computed(() => preview.value?.previewType())

const index = computed(() => {
  return preview.value.getIndex()
})

const total = computed(() => {
  return preview.value.getTotal()
})

const previousId = computed(() => {
  return preview.value.getPreviousId()
})

const nextId = computed(() => {
  return preview.value.getNextId()
})

/**
 * Close and destroy the modal.
 */
const cancel = () => {
  emits('cancel', preview.value)
}

/**
 * Start the download of a file through the
 * regular download process.
 */
const download = () => {
  if (!preview.value) return
  emits('download', preview.value)
}

/**
 * Remove the file from the Storage.
 */
const remove = () => {
  if (!preview.value) return
  emits('remove', preview.value)
}

/**
 * Remove the file from the Storage.
 */
const details = () => {
  if (!props.modelValue) return
  emits('details', props.modelValue)
}

/**
 * Keydown event handler
 */
const previewKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && !props.hideClose) {
    cancel()
  }

  if (e.key === 'ArrowLeft') {
    const previousId = preview.value.getPreviousId()

    if (previousId) emits('previous', previousId)
  }

  if (e.key === 'ArrowRight') {
    const nextId = preview.value.getNextId()

    if (nextId) emits('next', nextId)
  }
}

onMounted(() => {
  window.addEventListener('keydown', previewKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', previewKeydown)
})
</script>

<template>
  <Suspense>
    <div
      v-if="preview"
      class="fixed top-0 left-0 flex flex-col items-center justify-center w-full h-full dark:bg-brownish-950 pt-20 pb-20"
    >
      <slot />
      <div class="absolute top-0 w-full">
        <div class="float-right space-x-4 p-4">
          <BaseButton
            v-if="!hideDelete"
            color="danger"
            :icon="mdiTrashCan"
            small
            @click="remove"
            name="preview-remove"
          />
          <BaseButton
            color="light"
            :icon="mdiInformationSlabCircleOutline"
            small
            @click="details"
            name="preview-details"
          />
          <BaseButton
            color="light"
            :icon="mdiDownload"
            small
            @click="download"
            name="preview-download"
          />
          <BaseButton
            v-if="!hideClose"
            color="light"
            :icon="mdiClose"
            small
            @click="cancel"
            name="preview-close"
          />
        </div>
        <div class="float-left space-x-4 p-4">
          <h1>{{ preview.name }}</h1>
        </div>
      </div>

      <div class="absolute top-12 w-full" v-if="!hidePreviousAndNext">
        <div class="flex justify-center space-x-4 p-4" v-if="preview.is()">
          <BaseButton
            :disabled="!previousId"
            color="dark"
            :icon="mdiArrowLeft"
            small
            title="Previous image"
            :to="{ name: 'file-preview', params: { id: previousId as string } }"
          />
          <span
            class="inline-flex justify-center items-center whitespace-nowrap transition-colors p-1"
          >
            {{ index + 1 }} / {{ total }}
          </span>
          <BaseButton
            :disabled="!nextId"
            color="dark"
            :icon="mdiArrowRight"
            small
            title="Next image"
            :to="{ name: 'file-preview', params: { id: nextId as string } }"
          />
        </div>
      </div>

      <PreviewImage v-if="previewType === 'image'" v-model="preview" />

      <div class="flex flex-col" v-else>
        <div class="mb-4 text-center">
          <BaseIcon :path="mdiFileOutline" :size="75" h="h-75" w="w-75" />
        </div>
        <div class="text-center">
          <span> No preview available 🥲 </span>
        </div>
      </div>
    </div>
  </Suspense>
</template>
