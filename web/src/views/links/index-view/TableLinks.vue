<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TableLinkRowWatcher from './TableLinkRowWatcher.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { mdiTrashCanOutline } from '@mdi/js'
import type { AppLink } from 'types'

const props = defineProps<{
  selected: AppLink[]
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
    return props.selected.find((link) => link.id === item.id)
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

const borderClass = 'sm:border-l sm:border-brownish-50 sm:dark:border-brownish-950'

const sizes = {
  checkbox: 'pl-2 pt-3 w-10 shrink-0',
  name: 'flex-1 p-2 pt-3 min-w-0 flex',
  size: 'hidden p-2 pt-3 md:block w-24 shrink-0',
  createdAt: 'hidden p-2 pt-3 sm:block w-44 shrink-0',
  expiresAt: 'hidden p-2 pt-3 xl:block w-28 shrink-0',
  buttons: 'w-10 p-2 shrink-0'
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

  <div class="bg-white dark:bg-brownish-900 rounded-lg border border-brownish-200/40 dark:border-brownish-700/40">
    <div class="w-full flex rounded-t-lg bg-brownish-100 dark:bg-brownish-950 border-b border-brownish-200 dark:border-brownish-700/40">
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
      class="w-full pt-20 rounded-b-lg bg-brownish-50 dark:bg-brownish-900 h-52 text-center"
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
  </div>
</template>
