<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import { mdiDelete } from '@mdi/js'
import type { Session } from 'types/admin/sessions'
import { computed } from 'vue'

const props = defineProps<{
  session: Session
}>()

const emits = defineEmits(['kill'])

const createdAt = computed(() => {
  return formatPrettyDate(props.session.created_at)
})

const updatedAt = computed(() => {
  return formatPrettyDate(props.session.updated_at)
})

const expiresAt = computed(() => {
  return formatPrettyDate(props.session.expires_at)
})
</script>
<template>
  <tr>
    <td>{{ session.email }}</td>
    <td>{{ session.ip }}</td>
    <td>{{ session.user_agent }}</td>
    <td>{{ createdAt }}</td>
    <td>{{ updatedAt }}</td>
    <td>{{ expiresAt }}</td>
    <td>{{ session.active ? 'no' : 'yes' }}</td>
    <td>
      <BaseButtonConfirm
        :icon="mdiDelete"
        @confirm="emits('kill', session)"
        label="Kill session"
        confirm-label="Confirm"
        color="danger"
        :disabled="!session.active"
      />
    </td>
  </tr>
</template>
