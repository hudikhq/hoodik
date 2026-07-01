<script setup lang="ts">
import {
  mdiAccountArrowRight,
  mdiAlertCircle,
  mdiChevronRight,
  mdiCogOutline,
  mdiContentCopy
} from '@mdi/js'

import BaseIcon from '@/components/ui/BaseIcon.vue'

import { formatRelative } from '!/index'

import { ACTION_LABELS } from './useShareHubAudit'
import type { RowDisclosure, RowVerificationState } from './useShareHubAudit'

import type { ShareEvent } from 'types'

defineProps<{
  row: ShareEvent
  sentence: string
  senderEmail: string
  recipientEmail: string
  state: RowVerificationState
  expanded: boolean
  tamperedHeadline: string
  disclosure: RowDisclosure
}>()

const emit = defineEmits<{
  toggle: [rowId: string]
  export: [row: ShareEvent]
}>()

function actionLabel(row: ShareEvent): string {
  return ACTION_LABELS[row.action] ?? row.action
}

function formatTimestamp(unixSeconds: number): string {
  return formatRelative(unixSeconds)
}
</script>

<template>
  <li
    class="p-3 sm:p-4 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700 text-sm"
    :data-testid="`share-hub-audit-row-${row.id}`"
  >
    <div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3">
      <div class="flex items-start gap-3 min-w-0">
        <BaseIcon
          :path="mdiAccountArrowRight"
          :size="20"
          class="mt-1 shrink-0 text-brownish-300 dark:text-brownish-200"
        />
        <div class="min-w-0">
          <div
            class="text-sm break-words"
            :data-testid="`share-hub-audit-row-${row.id}-sentence`"
          >
            {{ sentence }}
          </div>
          <div
            class="text-xs text-brownish-400 mt-0.5"
            :title="new Date(row.created_at * 1000).toLocaleString()"
          >
            {{ formatTimestamp(row.created_at) }}
          </div>
          <!-- Hidden testids so existing e2e + vitest harnesses can
               still locate the sender / recipient labels without
               parsing the sentence string. -->
          <span
            class="sr-only"
            :data-testid="`share-hub-audit-row-${row.id}-sender`"
          >
            {{ senderEmail }}
          </span>
          <span
            class="sr-only"
            :data-testid="`share-hub-audit-row-${row.id}-recipient`"
          >
            {{ recipientEmail }}
          </span>
          <span
            class="sr-only"
            :data-testid="`share-hub-audit-row-${row.id}-action`"
          >
            {{ actionLabel(row) }}
          </span>
        </div>
      </div>
      <div class="flex flex-row sm:flex-col flex-wrap sm:items-end items-start gap-1.5 shrink-0">
        <!-- Tri-state badge:
             - verified: silent (the row IS the signal — no decoration)
             - system: single neutral pill ("System")
             - tampered: only via the row-level banner below
             + per-row chevron disclosure for the verification breakdown. -->
        <span
          v-if="state === 'system'"
          class="inline-flex items-center text-[11px] uppercase tracking-wider px-2 py-0.5 rounded-full bg-brownish-200 dark:bg-brownish-700 text-brownish-700 dark:text-brownish-200"
          :data-testid="`share-hub-audit-row-${row.id}-system`"
          :title="row.sender_id ? `Parent: ${row.sender_id}` : 'System cascade event'"
        >
          <BaseIcon :path="mdiCogOutline" :size="12" class="mr-1" />
          System
        </span>
        <button
          type="button"
          class="inline-flex items-center text-brownish-300 hover:text-brownish-100 transition"
          :title="expanded ? 'Hide verification details' : 'Show verification details'"
          :data-testid="`share-hub-audit-row-${row.id}-toggle`"
          @click="emit('toggle', row.id)"
        >
          <BaseIcon
            :path="mdiChevronRight"
            :size="18"
            :class="{ 'rotate-90': expanded }"
            class="transition-transform"
          />
        </button>
      </div>
    </div>

    <!-- Tampered banner — fires when any of the local checks fails on
         a non-system row: signature verification, self-hash recompute,
         or the chain link between two visible adjacent rows. Stays put
         until reload, no dismiss. The forensic CTA copies the row +
         computed verification breakdown to the clipboard. -->
    <div
      v-if="state === 'tampered'"
      class="mt-3 -mx-3 sm:-mx-4 -mb-3 sm:-mb-4 px-3 sm:px-4 py-3 bg-redish-100 dark:bg-redish-900/40 border-t border-redish-300 dark:border-redish-700 rounded-b-lg"
      :data-testid="`share-hub-audit-row-${row.id}-tampered-banner`"
    >
      <div class="flex items-start gap-2 text-redish-700 dark:text-redish-200">
        <BaseIcon :path="mdiAlertCircle" :size="20" class="shrink-0 mt-0.5" />
        <div class="flex-1 min-w-0">
          <p class="font-semibold text-sm">{{ tamperedHeadline }}</p>
          <p class="text-xs mt-0.5">Export the row below and share it with the project maintainer.</p>
          <button
            type="button"
            class="mt-1 inline-flex items-center gap-1 text-xs underline hover:no-underline"
            :data-testid="`share-hub-audit-row-${row.id}-export`"
            @click="emit('export', row)"
          >
            <BaseIcon :path="mdiContentCopy" :size="12" />
            Export this row for review
          </button>
        </div>
      </div>
    </div>

    <!-- Disclosure: per-row verification breakdown. Default closed;
         power users / security reviewers expand for hashes + status
         lines. Replaces the old always-on badge wall. -->
    <div
      v-if="expanded"
      class="mt-3 p-3 rounded-lg bg-brownish-100/60 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700 text-xs space-y-1.5 font-mono text-brownish-600 dark:text-brownish-200"
      :data-testid="`share-hub-audit-row-${row.id}-disclosure`"
    >
      <p>
        <span class="text-brownish-400 dark:text-brownish-300">row id</span>
        <span class="ml-2">…{{ disclosure.rowIdShort }}</span>
      </p>
      <p>
        <span class="text-brownish-400 dark:text-brownish-300">this hash</span>
        <span class="ml-2">…{{ disclosure.thisHashTail }}</span>
      </p>
      <p>
        <span class="text-brownish-400 dark:text-brownish-300">prev hash</span>
        <span class="ml-2">…{{ disclosure.prevHashTail }}</span>
      </p>
      <p v-if="disclosure.senderSigStatus">
        <span class="text-brownish-400 dark:text-brownish-300">signature</span>
        <span class="ml-2">{{ disclosure.senderSigStatus }}</span>
      </p>
      <p v-if="disclosure.chainStatus">
        <span class="text-brownish-400 dark:text-brownish-300">chain</span>
        <span class="ml-2">{{ disclosure.chainStatus }}</span>
      </p>
    </div>
  </li>
</template>
