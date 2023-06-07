<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TableLinkRowWatcher from './TableLinkRowWatcher.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { mdiTrashCanOutline } from '@mdi/js'
import type { AppLink } from 'types'

const props = defineProps<{
  forDelete: AppLink[]
  items: AppLink[]
  searchedFileId?: string
  hideCheckbox?: boolean
  loading?: boolean
}>()

const emits = defineEmits<{
  (event: 'link', item: AppLink): void
  (event: 'remove-all', items: AppLink[]): void
  (event: 'select-one', select: boolean, item: AppLink): void
  (event: 'select-all', items: AppLink[]): void
  (event: 'deselect-all'): void
}>()

const checked = ref(false)

const checkedRows = computed(() => {
  return props.items.filter((item) => {
    return props.forDelete.find((link) => link.id === item.id)
  })
})

const showDeleteAll = computed(() => {
  return checked.value || checkedRows.value.length > 0
})

watch(
  () => checkedRows.value,
  (value) => {
    if (value.length === 0) {
      checked.value = false
    }
  }
)

watch(
  () => checked.value,
  (value) => {
    if (value) {
      emits('select-all', props.items)
    } else {
      emits('select-all', [])
    }
  }
)

const borderClass = 'sm:border-l-2 sm:border-brownish-50 sm:dark:border-brownish-900'

const sizes = {
  checkbox: 'pl-2 pt-3 w-10',
  name: 'w-10/12 p-2 pt-3 sm:w-7/12 md:w-5/12 lg:w-6/12 xl:w-7/12 flex',
  size: 'hidden p-2 pt-3 md:block md:w-2/12 xl:w-1/12',
  createdAt: 'hidden p-2 pt-3 sm:block sm:w-4/12 lg:w-3/12 xl:w-2/12',
  expiresAt: 'hidden p-2 pt-3 xl:block xl:w-1/12',
  buttons: 'w-2/12 p-2 sm:w-1/12'
}
</script>

<template>
  <div class="w-full p-2 mb-2 flex rounded-t-md bg-brownish-100 dark:bg-brownish-900 gap-4">
    <BaseButton
      title="Delete selected links and folders"
      :iconSize="20"
      :xs="true"
      :icon="mdiTrashCanOutline"
      color="danger"
      :disabled="!showDeleteAll"
      @click="() => emits('remove-all', checkedRows)"
    />
  </div>

  <div class="w-full flex rounded-t-lg bg-brownish-100 dark:bg-brownish-950">
    <div :class="sizes.checkbox">
      <TableCheckboxCell v-model="checked" v-if="!props.hideCheckbox" />
    </div>

    <div :class="`${sizes.name}`">
      <span>Name</span>
    </div>

    <div :class="`${sizes.size} ${borderClass}`">
      <span>Size</span>
    </div>

    <div :class="`${sizes.createdAt} ${borderClass}`">
      <span>Created</span>
    </div>

    <div :class="`${sizes.expiresAt} ${borderClass}`">
      <span>Expires</span>
    </div>

    <div :class="`${sizes.buttons}`"></div>
  </div>

  <div
    v-if="props.loading"
    class="w-full pt-20 rounded-b-lg bg-brownish-100 dark:bg-brownish-900 h-52 text-center"
  >
    <span class="w-1/2 h-1/2">
      <SpinnerIcon :size="200" />
    </span>
  </div>
  <div v-else class="flex flex-col rounded-b-lg">
    <template v-for="link in items" :key="link.id">
      <TableLinkRowWatcher
        :link="link"
        :sizes="sizes"
        :checkedRows="checkedRows"
        :highlighted="props.searchedFileId === link.id"
        @link="(f: AppLink) => emits('link', f)"
        @select-one="(v: boolean, f: AppLink) => emits('select-one', v, f)"
        @deselect-all="emits('deselect-all')"
      />
    </template>
  </div>
</template>
