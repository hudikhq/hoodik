<script setup lang="ts">
import {
  mdiTrashCan,
  mdiDownload,
  mdiFileOutline,
  mdiClose,
  mdiPlus,
  mdiMinus,
  mdiInformationSlabCircleOutline
} from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import OverlayLayer from '@/components/ui/OverlayLayer.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import type { FilesStore, KeyPair, ListAppFile } from 'types'
import { computed, ref, watch } from 'vue'

const props = defineProps<{
  modelValue: ListAppFile | undefined
  hideDelete?: boolean
  storage: FilesStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
  (event: 'download', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
  (event: 'details', file: ListAppFile): void
}>()

const container = ref()
const imageUrl = ref<string>()
const imageW = ref(0)
const imageH = ref(0)
const scaleW = ref(0)
const scaleH = ref(0)

const file = computed({
  get: () => props.modelValue,
  set: (value: ListAppFile | undefined) => emits('update:modelValue', value)
})

const hasPreview = computed(() => !!file.value?.metadata?.thumbnail)

/**
 * Load the file data
 */
const get = async () => {
  if (!file.value) return

  if (hasPreview.value) {
    return load()
  }
}

/**
 * Load the binary data of the file from the backend
 */
const load = async () => {
  if (!file.value) return

  await fitUrl(file.value.metadata?.thumbnail)

  if (!file.value.data) {
    const { data } = await props.storage.get(file.value, props.kp)
    file.value.data = data
  }

  if (!file.value.data) return

  const blob = new Blob([file.value.data], { type: file.value.mime })
  const url = URL.createObjectURL(blob)

  await fitUrl(url)

  imageUrl.value = url
}

/**
 * Close and destroy the modal.
 */
const cancel = () => {
  file.value = undefined
  imageUrl.value = undefined
  imageW.value = 0
  imageH.value = 0
  scaleW.value = 0
  scaleH.value = 0
}

/**
 * Start the download of a file through the
 * regular download process.
 */
const download = () => {
  if (!props.modelValue) return
  emits('download', props.modelValue)
}

/**
 * Remove the file from the storage.
 */
const remove = () => {
  if (!props.modelValue) return
  emits('remove', props.modelValue)
}

/**
 * Remove the file from the storage.
 */
const details = () => {
  if (!props.modelValue) return
  emits('details', props.modelValue)
}

/**
 * Add to the image scale
 */
const plus = () => {
  scaleW.value *= 1.25
  scaleH.value *= 1.25
}

/**
 * Deduct from the image scale
 */
const minus = () => {
  scaleW.value *= 0.75
  scaleH.value *= 0.75
}

/**
 * Fit given URL to the container
 */
const fitUrl = (url: string) =>
  new Promise((resolve) => {
    const img = new Image()
    img.onload = () => {
      imageW.value = img.width
      imageH.value = img.height

      resolve(fit())
    }
    img.src = url
  })

/**
 * Fit the image to display fully within the available space
 */
const fit = () => {
  if (!container.value) return

  const aspectRatio = imageW.value / imageH.value
  const containerWidth = container.value.offsetWidth * 0.75
  const containerHeight = container.value.offsetHeight * 0.75
  const containerAspectRatio = containerWidth / containerHeight

  if (aspectRatio > containerAspectRatio) {
    scaleW.value = containerWidth
    scaleH.value = containerWidth / aspectRatio
  } else {
    scaleH.value = containerHeight
    scaleW.value = containerHeight * aspectRatio
  }
}

/**
 * Percentage of the image scale
 */
const percentage = computed(() => {
  if (imageW.value === 0 || scaleW.value === 0) return -1

  return Math.round((scaleW.value / imageW.value) * 100)
})

watch(
  () => props.modelValue,
  () => get(),
  { immediate: true }
)

window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && props.modelValue) {
    cancel()
  }

  if (e.key === '+' && props.modelValue) {
    plus()
  }

  if (e.key === '-' && props.modelValue) {
    minus()
  }
  if (e.key === ' ' && props.modelValue) {
    fit()
  }
})
</script>

<template>
  <OverlayLayer z-index="z-40" :visible="!!file">
    <div
      v-if="file"
      class="fixed top-0 left-0 flex flex-col items-center justify-center w-full h-full dark:bg-brownish-950 pt-20 pb-20"
    >
      <div class="absolute top-0 w-full">
        <div class="float-right space-x-4 p-4">
          <BaseButton v-if="!hideDelete" color="danger" :icon="mdiTrashCan" small @click="remove" />
          <BaseButton
            color="light"
            :icon="mdiInformationSlabCircleOutline"
            small
            @click="details"
          />
          <BaseButton color="light" :icon="mdiDownload" small @click="download" />
          <BaseButton color="light" :icon="mdiClose" small @click="cancel" />
        </div>
        <div class="float-left space-x-4 p-4">
          <h1>{{ file.metadata?.name }}</h1>
        </div>
      </div>

      <div ref="container" class="w-[100%] h-[calc(100%+2rem)] image-container">
        <template v-if="hasPreview">
          <img
            key="original"
            v-if="imageUrl"
            :src="imageUrl"
            :alt="props.modelValue?.metadata?.name"
            :height="scaleH"
            :width="scaleW"
          />
          <img
            key="thumbnail"
            v-else
            :src="props.modelValue?.metadata?.thumbnail"
            :alt="props.modelValue?.metadata?.name"
            :height="scaleH"
            :width="scaleW"
          />
        </template>
        <div class="flex flex-col" v-else>
          <div class="mb-4 text-center">
            <BaseIcon :path="mdiFileOutline" :size="75" h="h-75" w="w-75" />
          </div>
          <div class="text-center">
            <span> No preview available 🥲 </span>
          </div>
        </div>
      </div>

      <div class="absolute bottom-0 w-full">
        <div class="flex justify-center space-x-4 p-4" v-if="hasPreview">
          <BaseButton
            :disabled="!imageUrl || percentage <= 1"
            color="dark"
            :icon="mdiMinus"
            small
            @click="minus"
            title="Decrease image size (+)"
          />
          <BaseButton
            :disabled="!imageUrl"
            color="dark"
            small
            @click="fit"
            title="Fit image to screen (space)"
            :label="percentage > -1 ? `${percentage}%` : ' '"
          />
          <BaseButton
            :disabled="!imageUrl"
            color="dark"
            :icon="mdiPlus"
            small
            @click="plus"
            title="Increase image size (+)"
          />
        </div>
      </div>
    </div>
  </OverlayLayer>
</template>
<style scoped lang="css">
.image-container {
  display: grid;
  align-content: center;
  justify-content: center;
  overflow: scroll;
}

.image-container img {
  max-width: 1000%;
  max-height: 1000%;
}
</style>
