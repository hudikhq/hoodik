<script setup lang="ts">
import { mdiPlus, mdiMinus } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import type { Preview } from '!/preview'
import SpinnerIcon from '../ui/SpinnerIcon.vue'

const props = defineProps<{
  modelValue: Preview
}>()
const preview = computed(() => props.modelValue)

const container = ref()
const imageUrl = ref<string>()
const imageW = ref(0)
const imageH = ref(0)
const scaleW = ref(0)
const scaleH = ref(0)

/**
 * Load the preview as image,
 *  - first, fit the thumbnail to the container so we don't have blank screen while loading...
 *  - then load the full image and fit it to the container.
 */
const load = async () => {
  if (preview.value.thumbnail) {
    await fitUrl(preview.value.thumbnail)
  }

  const blob = new Blob([await preview.value.load()], { type: preview.value.mime })
  const url = URL.createObjectURL(blob)

  await fitUrl(url)

  imageUrl.value = url
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

/**
 * Keydown event handler
 */
const imageKeydown = (e: KeyboardEvent) => {
  if (e.key === '+') {
    plus()
  }

  if (e.key === '-') {
    minus()
  }

  if (e.key === ' ') {
    fit()
  }
}

onMounted(() => {
  window.addEventListener('keydown', imageKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', imageKeydown)
})

watch(
  () => props.modelValue,
  () => setTimeout(load, 100),
  { immediate: true }
)
</script>

<template>
  <div ref="container" class="w-[100%] h-[calc(100%+2rem)] image-container">
    <img
      v-if="imageUrl"
      key="original"
      name="original"
      :src="imageUrl"
      :alt="preview.name"
      :height="scaleH"
      :width="scaleW"
    />
    <img
      v-else-if="preview.thumbnail"
      key="thumbnail"
      name="loading-thumbnail"
      :src="preview.thumbnail"
      :alt="preview.name"
      :height="scaleH"
      :width="scaleW"
    />
    <div v-else class="flex justify-center items-center w-full h-full">
      <SpinnerIcon />
    </div>
  </div>

  <div class="absolute bottom-0 w-full">
    <div class="flex justify-center space-x-4 p-4 pb-6">
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
