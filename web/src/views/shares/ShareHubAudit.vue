<script setup lang="ts">
import { onMounted, toRef } from 'vue'

import { mdiAlertCircleOutline } from '@mdi/js'

import BaseIcon from '@/components/ui/BaseIcon.vue'

import ShareHubAuditFilters from './ShareHubAuditFilters.vue'
import ShareHubAuditRow from './ShareHubAuditRow.vue'
import { useShareHubAudit } from './useShareHubAudit'

import type { KeyPair } from 'types'

const props = defineProps<{
  authenticated?: { user: { id: string; email: string } } | null
  keypair?: KeyPair
}>()

const {
  fileIdFilter,
  actionFilter,
  senderEmailInput,
  senderError,
  senderResolving,
  recipientFilter,
  startDate,
  endDate,
  loading,
  filteredRows,
  hasActiveFilter,
  refresh,
  rowSentence,
  senderEmail,
  recipientEmail,
  rowState,
  tamperedHeadline,
  rowDisclosure,
  toggleDisclosure,
  isExpanded,
  exportRow,
  clearFilters,
  resolveSenderEmail
} = useShareHubAudit(toRef(props, 'keypair'))

onMounted(refresh)
</script>

<template>
  <div data-testid="share-hub-audit">
    <ShareHubAuditFilters
      v-model:file-id-filter="fileIdFilter"
      v-model:action-filter="actionFilter"
      v-model:sender-email-input="senderEmailInput"
      v-model:recipient-filter="recipientFilter"
      v-model:start-date="startDate"
      v-model:end-date="endDate"
      :sender-error="senderError"
      :sender-resolving="senderResolving"
      @resolve-sender="resolveSenderEmail"
      @clear="clearFilters"
    />

    <p v-if="loading" class="text-sm text-brownish-300" data-testid="share-hub-audit-loading">
      Loading events…
    </p>

    <!-- Filter banner — the chain-walk skips paging gaps inside the
         loaded slice, but a user-supplied filter (file/sender/recipient/
         date) narrows the slice further. The banner makes that obvious
         in a quiet info treatment, no red.  -->
    <div
      v-if="hasActiveFilter"
      class="p-3 mb-3 rounded-lg bg-brownish-100 dark:bg-brownish-900/60 text-sm flex items-start gap-2 text-brownish-600 dark:text-brownish-200 border border-brownish-200 dark:border-brownish-700"
      data-testid="share-hub-audit-filter-banner"
    >
      <BaseIcon :path="mdiAlertCircleOutline" :size="16" class="shrink-0 mt-0.5" />
      <span>Filtered view — chain verification limited to visible rows.</span>
    </div>

    <ul
      v-if="filteredRows.length"
      class="space-y-2"
      data-testid="share-hub-audit-list"
    >
      <ShareHubAuditRow
        v-for="row in filteredRows"
        :key="row.id"
        :row="row"
        :sentence="rowSentence(row)"
        :sender-email="senderEmail(row)"
        :recipient-email="recipientEmail(row)"
        :state="rowState(row)"
        :expanded="isExpanded(row.id)"
        :tampered-headline="tamperedHeadline(row)"
        :disclosure="rowDisclosure(row)"
        @toggle="toggleDisclosure"
        @export="exportRow"
      />
    </ul>

    <p
      v-else-if="!loading"
      class="p-6 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700 text-sm text-brownish-300"
      data-testid="share-hub-audit-empty"
    >
      No events match the current filters.
    </p>
  </div>
</template>
