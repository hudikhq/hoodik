<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import { mdiDelete } from '@mdi/js'
import type { Authenticated, Session } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  authenticated: Authenticated
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

const expired = computed(() => {
  return props.session.expires_at < new Date().valueOf() / 1000
})
</script>
<template>
  <tr>
    <td>{{ session.ip }}</td>
    <td>{{ session.user_agent }}</td>
    <td>{{ createdAt }}</td>
    <td>{{ updatedAt }}</td>
    <td class="text-greeny-500" v-if="authenticated.session.id === session.id">current</td>
    <td v-else-if="expired">expired</td>
    <td v-else>
      {{ expiresAt }}
    </td>
    <td>{{ session.refresh ? 'no' : 'yes' }}</td>
    <td>
      <BaseButtonConfirm
        :icon="mdiDelete"
        @confirm="emits('kill', session)"
        label="Kill activity"
        confirm-label="Confirm"
        color="danger"
        :disabled="!session.refresh || authenticated.session.id === session.id"
      />
    </td>
  </tr>
</template>
