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

const invitation = computed(() => {
  return props.invitation
})

const createdAt = computed(() => {
  return formatPrettyDate(invitation.value.created_at)
})

const expiresAt = computed(() => {
  return formatPrettyDate(invitation.value.expires_at)
})

const isExpired = computed(() => {
  const now = new Date().valueOf() / 1000
  return invitation.value.expires_at < now
})
</script>
<template>
  <tr :title="invitation.message">
    <td>{{ invitation.email }}</td>
    <td>{{ createdAt }}</td>
    <td>{{ expiresAt }}</td>
    <td class="text-right">
      <BaseButtonConfirm
        :icon="mdiDelete"
        @confirm="emits('expire', invitation)"
        label="Expire"
        confirm-label="Confirm"
        color="danger"
        :disabled="isExpired"
      />
    </td>
  </tr>
</template>
