<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import type { Invitation } from 'types/admin/invitations'
import { computed } from 'vue'
import { mdiDelete } from '@mdi/js'

const props = defineProps<{
  invitation: Invitation
}>()

const emits = defineEmits(['expire'])

const createdAt = computed(() => formatPrettyDate(props.invitation.created_at))
const expiresAt = computed(() => formatPrettyDate(props.invitation.expires_at))

const isExpired = computed(() => {
  const now = new Date().valueOf() / 1000
  return props.invitation.expires_at < now
})
</script>
<template>
  <tr :title="invitation.message" :class="{ 'opacity-50': isExpired }">
    <td data-label="Email">{{ invitation.email }}</td>
    <td data-label="Sent">{{ createdAt }}</td>
    <td data-label="Expires">{{ expiresAt }}</td>
    <td data-label="Status">
      <span
        v-if="isExpired"
        class="inline-flex items-center text-xs font-medium bg-brownish-100 dark:bg-brownish-700 text-brownish-400 px-2 py-0.5 rounded-full"
      >expired</span>
      <span
        v-else
        class="inline-flex items-center text-xs font-medium bg-greeny-500/15 text-greeny-500 dark:text-greeny-400 px-2 py-0.5 rounded-full"
      >pending</span>
    </td>
    <td>
      <BaseButtonConfirm
        :icon="mdiDelete"
        @confirm="emits('expire', invitation)"
        label="Revoke"
        confirm-label="Yes, revoke"
        color="danger"
        :small="true"
        :disabled="isExpired"
      />
    </td>
  </tr>
</template>
