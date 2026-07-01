<script setup lang="ts">
import type { AppFile } from 'types'
import BaseButton from '../ui/BaseButton.vue'
import { computed } from 'vue'
import { SHARED_WITH_ME_DIR_ID } from '!/storage'

const props = defineProps<{
  parents: AppFile[]
  parentId?: string
}>()

/**
 * The root crumb reflects the surface the user is browsing inside. When
 * the chain originates from incoming content (the synthetic
 * `__shared_with_me__` placeholder or any ancestor the caller doesn't own)
 * the recipient lands on "Shared with me"; owned content keeps "My Files".
 * Both root anchors route to their respective entry points so the user can
 * jump back without leaving the file browser.
 */
const inSharedContent = computed<boolean>(() => {
  const head = props.parents[0]
  if (!head) return false
  if (head.id === SHARED_WITH_ME_DIR_ID) return true
  return head.is_owner === false
})

const rootCrumb = computed(() =>
  inSharedContent.value
    ? {
        label: 'Shared with me',
        to: { name: 'files', params: { file_id: SHARED_WITH_ME_DIR_ID } }
      }
    : {
        label: 'My Files',
        to: { name: 'files' }
      }
)

/**
 * The chain begins with either the synthetic root or the recipient's
 * shared-content head. Both already render through `rootCrumb`, so drop
 * them from the per-parent iteration to avoid duplicating the entry.
 */
const visibleParents = computed<AppFile[]>(() => {
  return props.parents.filter((p) => p.id !== SHARED_WITH_ME_DIR_ID)
})

const skipIndexes = computed<number[]>(() => {
  if (visibleParents.value.length < 3) {
    return []
  }
  return visibleParents.value
    .slice(1, visibleParents.value.length - 2)
    .map((_, index) => index + 1)
})
</script>

<template>
  <nav aria-label="Breadcrumb">
    <ol class="flex mb-2">
      <li>
        <BaseButton
          :to="rootCrumb.to"
          :label="rootCrumb.label"
          :disabled="!visibleParents.length"
          class="pl-1 pr-1 text-lg"
          data-testid="breadcrumb-root"
        />
      </li>

      <template v-for="(parent, index) in visibleParents" v-bind:key="index">
        <li v-if="skipIndexes.indexOf(index) === -1">
          <span> / </span>
          <BaseButton
            :to="{ name: 'files', params: { file_id: parent.id } }"
            :label="`${parent.name || '...'}`"
            class="pl-1 pr-1 text-lg"
          />
        </li>
        <li v-else-if="skipIndexes[0] === index">
          <span> / </span>
          <span
            class="inline-flex justify-center items-center whitespace-nowrap focus:outline-none transition-colors focus:ring duration-150 rounded border-brownish-100 dark:border-brownish-800 ring-brownish-200 dark:ring-brownish-500 bg-brownish-100 text-black dark:bg-brownish-800 dark:text-white border py-2 px-3 text-lg"
          >
            ...
          </span>
        </li>
      </template>
    </ol>
  </nav>
</template>
