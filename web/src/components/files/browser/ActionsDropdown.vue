<script setup lang="ts">
import type { ListAppFile } from 'types'
import { mdiDotsVertical } from '@mdi/js'
import ActionsButtons from './ActionsButtons.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { ref, computed } from 'vue'
import { vOnClickOutside } from '@vueuse/components'

const props = defineProps<{
  modelValue: ListAppFile
  hideDelete?: boolean
  share?: boolean
  disabled?: boolean
  class?: string
}>()

const tracker = ref()
const menu = ref()
const menuWidth = ref(0)
const menuHeight = ref(0)

const emits = defineEmits<{
  (event: 'remove', file: ListAppFile): void
  (event: 'details', file: ListAppFile): void
  (event: 'link', file: ListAppFile): void
  (event: 'preview', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
  (event: 'update:modelValue', value: ListAppFile): void
}>()

const file = computed({
  get: () => props.modelValue,
  set: (v: ListAppFile) => emits('update:modelValue', v)
})

const remove = () => {
  close()
  emits('remove', file.value)
}

const details = () => {
  close()
  emits('details', file.value)
}

const link = () => {
  close()
  emits('link', file.value)
}

const preview = () => {
  close()
  emits('preview', file.value)
}

const download = () => {
  close()
  emits('download', file.value)
}

/**
 * Handle closing the dropdown
 */
const close = () => {
  if (menu.value?.style?.display === 'block') {
    menu.value.style.display = 'none'
  }
}

/**
 * Handle opening the context menu
 */
const open = (event: MouseEvent) => {
  if (props.disabled) {
    return
  }

  if (!tracker.value) {
    return
  }

  if (!menu.value) {
    return
  }

  if (menu.value?.style?.display === 'block') {
    return
  }

  if (!menuWidth.value || !menuHeight.value) {
    menu.value.style.visibility = 'hidden'
    menu.value.style.display = 'block'
    menuWidth.value = menu.value.offsetWidth
    menuHeight.value = menu.value.offsetHeight
    menu.value.removeAttribute('style')
  }

  if (menuWidth.value + event.pageX >= window.innerWidth) {
    menu.value.style.left = event.pageX - menuWidth.value + 2 + 'px'
  } else {
    menu.value.style.left = event.pageX - 2 + 'px'
  }

  if (menuHeight.value + event.pageY >= window.innerHeight) {
    menu.value.style.top = event.pageY - menuHeight.value + 2 + 'px'
  } else {
    menu.value.style.top = event.pageY - 2 + 'px'
  }

  menu.value.style.display = 'block'
}

window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    close()
  }
})
</script>
<template>
  <div ref="tracker" v-on-click-outside="close" :class="props.class" :key="`${file.id}-tracker`">
    <slot>
      <BaseButton
        class="ml-2 hidden sm:block float-right"
        color="dark"
        :icon="mdiDotsVertical"
        small
        :disabled="props.disabled"
        @click="open"
      />
    </slot>
    <div
      ref="menu"
      :key="`${file.id}-menu`"
      class="floating-menu bg-brownish-700 shadow-lg max-h-modal w-8/12 md:w-3/5 lg:w-2/5 xl:w-1/12 z-50 py-2 m-4"
    >
      <ActionsButtons
        v-model="file"
        :hide-delete="hideDelete"
        :share="share"
        @remove="remove"
        @details="details"
        @link="link"
        @preview="preview"
        @download="download"
      />
    </div>
  </div>
</template>
<style scoped lang="postcss">
.floating-menu {
  display: none;
  position: absolute;
  top: 0;
  left: 0;
  margin: 0;
  padding: 0;
  z-index: 1000000;
}
</style>
