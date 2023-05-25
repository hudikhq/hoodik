<script setup lang="ts">
import { mdiDotsVertical } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TruncatedSpan from '@/components/ui/TruncatedSpan.vue'
import { formatPrettyDate, formatSize } from '!'
import type { AppLink } from 'types'
import { computed, ref } from 'vue'

const props = defineProps<{
  link: AppLink
  checkedRows: Partial<AppLink>[]
  highlighted?: boolean
  sizes: {
    checkbox: string
    name: string
    size: string
    createdAt: string
    fileCreatedAt: string
    expiresAt: string
    buttons: string
  }
}>()

const emits = defineEmits<{
  (event: 'link', link: AppLink): void
  (event: 'remove', link: AppLink): void
  (event: 'select-one', value: boolean, link: AppLink): void
}>()

const selectOne = (value: boolean) => {
  emits('select-one', value, props.link)
}

const checked = computed({
  get: () => !!props.checkedRows.find((item) => item.id === props.link.id),
  set: (v) => selectOne(v)
})

const linkName = computed(() => {
  return props.link.name || '...'
})

const linkSize = computed(() => {
  return props.link.file_size ? formatSize(props.link.file_size) : ''
})

const linkCreatedAt = computed(() => {
  return props.link.created_at ? formatPrettyDate(props.link.created_at) : ''
})

const fileCreatedAt = computed(() => {
  return props.link.file_created_at ? formatPrettyDate(props.link.file_created_at) : ''
})

const linkExpiresAt = computed(() => {
  return props.link.expires_at ? formatPrettyDate(props.link.expires_at) : ''
})

const sharedClass = computed(() => {
  return 'dark:bg-brownish-900 hover:bg-dirty-white hover:dark:bg-brownish-700'
})

const border = 'sm:border-l-2 sm:border-brownish-50 sm:dark:border-brownish-950'
const sizes = computed(() => {
  return {
    checkbox: `${props.sizes.checkbox}`,
    name: `${props.sizes.name}`,
    size: `${border} ${props.sizes.size}`,
    createdAt: `${border} ${props.sizes.createdAt}`,
    fileCreatedAt: `${border} ${props.sizes.fileCreatedAt}`,
    expiresAt: `${border} ${props.sizes.expiresAt}`,
    buttons: `${props.sizes.buttons} text-right`
  }
})

const clicks = ref(0)
const timer = ref()

/**
 * Click listener
 * that handles single and double clicks
 */
const click = () => {
  clicks.value++
  if (clicks.value === 1) {
    timer.value = setTimeout(() => {
      clicks.value = 0
      singleClick()
    }, 200)
  } else {
    clearTimeout(timer.value)
    clicks.value = 0
    doubleClick()
  }
}

const singleClick = () => {
  emits('link', props.link)
}

const doubleClick = () => {
  checked.value = !checked.value
}
</script>

<template>
  <div
    class="w-full flex"
    :class="{
      'bg-greeny-100 dark:bg-greeny-900 hover:bg-greeny-200 hover:dark:bg-greeny-800':
        props.highlighted,
      [sharedClass]: true
    }"
  >
    <div :class="sizes.checkbox">
      <TableCheckboxCell v-model="checked" />
    </div>

    <button
      :class="`${sizes.name} flex justify-start cursor-pointer prevent-select text-left`"
      :title="linkName"
      @click="click"
    >
      <img
        name="thumbnail"
        v-if="link.thumbnail"
        :src="link.thumbnail"
        :alt="linkName"
        class="w-6 h-6 mr-2 rounded-md"
      />

      <TruncatedSpan :text="linkName" />
    </button>

    <div :class="sizes.size" :title="linkSize">
      <span>{{ linkSize || '-' }}</span>
    </div>

    <div :class="sizes.createdAt" :title="props.link.created_at">
      <TruncatedSpan :text="linkCreatedAt" />
    </div>

    <div :class="sizes.fileCreatedAt" :title="props.link.created_at">
      <TruncatedSpan :text="fileCreatedAt" />
    </div>

    <div :class="sizes.expiresAt" :title="props.link.expires_at">
      <TruncatedSpan :text="linkExpiresAt" />
    </div>

    <div :class="sizes.buttons">
      <BaseButton
        class="ml-2 sm:hidden float-right"
        color="dark"
        :icon="mdiDotsVertical"
        small
        @click="emits('link', link)"
        :disabled="!props.link.id"
      />
    </div>
  </div>
</template>
<style lang="css">
.prevent-select {
  -webkit-user-select: none;
  -ms-user-select: none;
  user-select: none;
}
</style>
