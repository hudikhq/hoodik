<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TableLinkRowWatcher from './TableLinkRowWatcher.vue'
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

const borderClass = 'sm:border-l sm:border-paper-300 sm:dark:border-brownish-950'

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
  <div class="w-full p-2 mb-2 flex rounded-t-md bg-paper-100 dark:bg-brownish-900 gap-4">
    <BaseButton
      :title="$t('links.table.deleteSelected')"
      :iconSize="20"
      :xs="true"
      :icon="mdiTrashCanOutline"
      color="danger"
      :disabled="!showDeleteAll"
      @click="() => emits('remove-all', checkedRows)"
    />
  </div>

  <div class="bg-white dark:bg-brownish-900 rounded-lg border border-paper-300/40 dark:border-brownish-700/40">
    <div class="w-full flex rounded-t-lg bg-paper-100 dark:bg-brownish-950 border-b border-paper-300 dark:border-brownish-700/40">
      <div :class="sizes.checkbox">
        <TableCheckboxCell v-model="checked" v-if="!props.hideCheckbox" :label="$t('common.selectAll')" />
      </div>

      <div :class="`${sizes.name}`">
        <span>{{ $t('common.name') }}</span>
      </div>

      <div :class="`${sizes.size} ${borderClass}`">
        <span>{{ $t('common.size') }}</span>
      </div>

      <div :class="`${sizes.createdAt} ${borderClass}`">
        <span>{{ $t('common.created') }}</span>
      </div>

      <div :class="`${sizes.expiresAt} ${borderClass}`">
        <span>{{ $t('links.expires') }}</span>
      </div>

      <div :class="`${sizes.buttons}`"></div>
    </div>

    <!-- Placeholder rows on the real columns, so the header does not jump when
         the links land — same treatment as the file listing. -->
    <div
      v-if="props.loading && !items.length"
      class="w-full rounded-b-lg bg-paper-50 dark:bg-brownish-900"
      data-testid="links-loading"
      role="status"
      :aria-label="$t('common.loading')"
    >
      <div
        v-for="(width, index) in ['w-1/2', 'w-2/5', 'w-3/5']"
        :key="index"
        class="w-full flex link-row-separator animate-pulse"
        aria-hidden="true"
      >
        <div :class="sizes.checkbox">
          <div class="w-5 h-5 rounded bg-paper-200 dark:bg-brownish-700" />
        </div>
        <div :class="sizes.name">
          <div class="w-6 h-6 mr-2 rounded-md shrink-0 bg-paper-200 dark:bg-brownish-700" />
          <div class="h-4 rounded bg-paper-200 dark:bg-brownish-700" :class="width" />
        </div>
        <div :class="sizes.size">
          <div class="h-3 w-12 rounded bg-paper-200/70 dark:bg-brownish-700/60" />
        </div>
        <div :class="sizes.createdAt">
          <div class="h-3 w-28 rounded bg-paper-200/70 dark:bg-brownish-700/60" />
        </div>
        <div :class="sizes.expiresAt">
          <div class="h-3 w-16 rounded bg-paper-200/70 dark:bg-brownish-700/60" />
        </div>
        <div :class="sizes.buttons" />
      </div>
    </div>
    <div
      v-else-if="!items.length"
      class="w-full rounded-b-lg bg-paper-50 dark:bg-brownish-900 py-14 flex flex-col items-center gap-1"
      data-testid="links-empty"
    >
      <span class="text-brownish-300 dark:text-brownish-50">{{ $t('links.table.empty') }}</span>
      <span class="text-xs text-brownish-200 dark:text-brownish-50">
        {{ $t('links.table.emptyHint') }}
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
