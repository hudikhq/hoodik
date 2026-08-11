<script setup lang="ts">
import type { AppFile } from 'types'
import BaseButton from '../ui/BaseButton.vue'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { SHARED_WITH_ME_DIR_ID } from '!/storage'

const { t } = useI18n()

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
        label: t('files.sharedWithMe'),
        to: { name: 'files', params: { file_id: SHARED_WITH_ME_DIR_ID } }
      }
    : {
        label: t('files.myFiles'),
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

const expanded = ref(false)

const skipIndexes = computed<number[]>(() => {
  if (expanded.value || visibleParents.value.length < 3) {
    return []
  }
  return visibleParents.value
    .slice(1, visibleParents.value.length - 2)
    .map((_, index) => index + 1)
})
</script>

<template>
  <nav :aria-label="$t('files.breadcrumbs')">
    <ol class="flex mb-2">
      <li>
        <BaseButton
          :to="rootCrumb.to"
          :label="rootCrumb.label"
          :disabled="!visibleParents.length"
          no-border
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
            no-border
            class="pl-1 pr-1 text-lg"
          />
        </li>
        <li v-else-if="skipIndexes[0] === index">
          <span> / </span>
          <BaseButton
            label="..."
            no-border
            :title="$t('files.breadcrumbsShowAll')"
            class="pl-1 pr-1 text-lg"
            data-testid="breadcrumb-expand"
            @click="expanded = true"
          />
        </li>
      </template>
    </ol>
  </nav>
</template>
