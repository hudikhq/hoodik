<script setup lang="ts">
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TruncatedSpan from '@/components/ui/TruncatedSpan.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { formatPrettyDate, formatSize } from '!'
import { mdiOpenInNew, mdiDownload } from '@mdi/js'
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
    expiresAt: string
    buttons: string
  }
}>()

const emits = defineEmits<{
  (event: 'link', link: AppLink): void
  (event: 'select-one', value: boolean, link: AppLink): void
  (event: 'deselect-all'): void
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
  return props.link.file_modified_at ? formatPrettyDate(props.link.file_modified_at) : ''
})

const linkExpiresAt = computed(() => {
  return props.link.expires_at ? formatPrettyDate(props.link.expires_at) : 'never'
})

const isExpired = computed(() => {
  const now = new Date()

  return props.link?.expires_at && new Date(props.link?.expires_at) < now
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
    singleClick()

    timer.value = setTimeout(() => {
      clicks.value = 0
    }, 250)
  }

  if (clicks.value === 2) {
    clicks.value = 0
    clearTimeout(timer.value)
    doubleClick()
  }
}

const doubleClick = () => {
  emits('link', props.link)
}

const singleClick = () => {
  emits('deselect-all')
  selectOne(!checked.value)
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

      <span class="flex ml-3">
        <BaseIcon :path="mdiDownload" :size="15" />

        {{ link.downloads }}
      </span>
    </button>

    <div :class="sizes.size" :title="linkSize">
      <TruncatedSpan :text="linkSize || '-'" />
    </div>

    <div :class="sizes.createdAt" :title="`File created: ${fileCreatedAt}`">
      <TruncatedSpan :text="linkCreatedAt" />
    </div>

    <div :class="sizes.expiresAt" :title="linkExpiresAt">
      <span v-if="isExpired" class="inline-block text-redish-200">expired</span>
      <TruncatedSpan v-else :text="linkExpiresAt" />
    </div>

    <div :class="sizes.buttons">
      <BaseButton
        title="View link"
        :icon="mdiOpenInNew"
        color="dark"
        small
        rounded-full
        class="ml-2 float-right"
        :to="{ name: 'links-view', params: { link_id: link.id }, hash: `#${link.link_key_hex}` }"
        target="_blank"
        v-if="link"
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
