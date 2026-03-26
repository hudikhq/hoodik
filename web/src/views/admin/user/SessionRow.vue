<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import { mdiShieldOffOutline } from '@mdi/js'
import type { Session } from 'types/admin/sessions'
import { computed } from 'vue'

const props = defineProps<{
  session: Session
}>()

const emits = defineEmits(['kill'])

const createdAt = computed(() => formatPrettyDate(props.session.created_at))
const updatedAt = computed(() => formatPrettyDate(props.session.updated_at))
const expiresAt = computed(() => formatPrettyDate(props.session.expires_at))
</script>
<template>
  <tr :class="{ 'opacity-50': !session.active }">
    <td data-label="Email">{{ session.email }}</td>
    <td data-label="IP Address">{{ session.ip }}</td>
    <td data-label="Device" class="max-w-[200px] truncate text-sm">{{ session.user_agent }}</td>
    <td data-label="Signed in">{{ createdAt }}</td>
    <td data-label="Last seen">{{ updatedAt }}</td>
    <td data-label="Expires">{{ expiresAt }}</td>
    <td data-label="Status">
      <span v-if="session.active" class="inline-flex items-center text-xs font-medium bg-blueish-500/15 text-blueish-400 px-2 py-0.5 rounded-full">active</span>
      <span v-else class="inline-flex items-center text-xs font-medium bg-brownish-100 dark:bg-brownish-700 text-brownish-400 px-2 py-0.5 rounded-full">revoked</span>
    </td>
    <td>
      <BaseButtonConfirm
        :icon="mdiShieldOffOutline"
        @confirm="emits('kill', session)"
        label="Revoke"
        confirm-label="Yes, revoke"
        color="danger"
        :small="true"
        :disabled="!session.active"
      />
    </td>
  </tr>
</template>
