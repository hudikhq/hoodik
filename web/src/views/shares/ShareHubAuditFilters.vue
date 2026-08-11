<script setup lang="ts">
import BaseButton from '@/components/ui/BaseButton.vue'

import { ACTION_LABEL_KEYS } from './useShareHubAudit'

import type { AuditEventAction } from 'types'

defineProps<{
  fileIdFilter: string
  actionFilter: 'all' | AuditEventAction
  senderEmailInput: string
  senderError: string | null
  senderResolving: boolean
  recipientFilter: string
  recipientOptions: { id: string; email: string }[]
  startDate: string
  endDate: string
}>()

const emit = defineEmits<{
  'update:fileIdFilter': [value: string]
  'update:actionFilter': [value: 'all' | AuditEventAction]
  'update:senderEmailInput': [value: string]
  'update:recipientFilter': [value: string]
  'update:startDate': [value: string]
  'update:endDate': [value: string]
  'resolve-sender': []
  clear: []
}>()
</script>

<template>
  <div
    class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-6 gap-3 mb-4 p-3 sm:p-4 rounded-lg bg-paper-50 dark:bg-brownish-900/60 border border-paper-300 dark:border-brownish-700"
  >
    <label class="text-xs sm:col-span-2 lg:col-span-2">
      <span class="block mb-1 text-brownish-300 dark:text-brownish-50">{{ $t('shares.audit.filters.fileId') }}</span>
      <input
        :value="fileIdFilter"
        type="text"
        placeholder="00000000-…"
        class="w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 font-mono focus:outline-none focus:border-redish-500"
        data-testid="share-hub-audit-file-filter"
        @input="emit('update:fileIdFilter', ($event.target as HTMLInputElement).value)"
      />
    </label>
    <label class="text-xs">
      <span class="block mb-1 text-brownish-300 dark:text-brownish-50">{{ $t('shares.audit.filters.action') }}</span>
      <select
        :value="actionFilter"
        data-testid="share-hub-audit-action-filter"
        class="w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500"
        @change="emit('update:actionFilter', ($event.target as HTMLSelectElement).value as 'all' | AuditEventAction)"
      >
        <option value="all">{{ $t('shares.audit.filters.allActions') }}</option>
        <option
          v-for="(labelKey, key) in ACTION_LABEL_KEYS"
          :key="key"
          :value="key"
        >{{ $t(labelKey) }}</option>
      </select>
    </label>
    <label class="text-xs">
      <span class="block mb-1 text-brownish-300 dark:text-brownish-50">{{ $t('shares.audit.filters.sender') }}</span>
      <div class="flex gap-2">
        <input
          :value="senderEmailInput"
          type="email"
          :placeholder="$t('shares.audit.filters.senderPlaceholder')"
          class="flex-1 min-w-0 bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500"
          data-testid="share-hub-audit-sender-filter"
          @input="emit('update:senderEmailInput', ($event.target as HTMLInputElement).value)"
          @keydown.enter.prevent="emit('resolve-sender')"
        />
        <button
          type="button"
          class="shrink-0 px-3 py-2 text-xs font-medium rounded-lg bg-paper-200 dark:bg-brownish-700 hover:bg-paper-300 dark:hover:bg-brownish-600 disabled:opacity-50"
          :disabled="senderResolving"
          data-testid="share-hub-audit-sender-resolve"
          @click.prevent="emit('resolve-sender')"
        >
          {{ $t('shares.audit.filters.find') }}
        </button>
      </div>
      <p
        v-if="senderError"
        class="mt-1 text-redish-600 dark:text-redish-100"
        data-testid="share-hub-audit-sender-error"
      >
        {{ senderError }}
      </p>
    </label>
    <label class="text-xs">
      <span class="block mb-1 text-brownish-300 dark:text-brownish-50">{{ $t('shares.audit.filters.recipient') }}</span>
      <select
        :value="recipientFilter"
        class="w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500"
        data-testid="share-hub-audit-recipient-filter"
        @change="emit('update:recipientFilter', ($event.target as HTMLSelectElement).value)"
      >
        <option value="">{{ $t('shares.audit.filters.allRecipients') }}</option>
        <option v-for="r in recipientOptions" :key="r.id" :value="r.id">{{ r.email }}</option>
      </select>
    </label>
    <label class="text-xs">
      <span class="block mb-1 text-brownish-300 dark:text-brownish-50">{{ $t('shares.audit.filters.from') }}</span>
      <input
        :value="startDate"
        type="date"
        class="w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500"
        data-testid="share-hub-audit-from"
        @input="emit('update:startDate', ($event.target as HTMLInputElement).value)"
      />
    </label>
    <label class="text-xs">
      <span class="block mb-1 text-brownish-300 dark:text-brownish-50">{{ $t('shares.audit.filters.to') }}</span>
      <input
        :value="endDate"
        type="date"
        class="w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500"
        data-testid="share-hub-audit-to"
        @input="emit('update:endDate', ($event.target as HTMLInputElement).value)"
      />
    </label>
    <div class="sm:col-span-2 lg:col-span-6 flex justify-end">
      <BaseButton
        color="info"
        small
        outline
        :label="$t('shares.audit.filters.clear')"
        data-testid="share-hub-audit-clear"
        @click.prevent="emit('clear')"
      />
    </div>
  </div>
</template>
