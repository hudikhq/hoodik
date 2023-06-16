<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButton from '@/components/ui/BaseButton.vue'
import type { User } from 'types/admin/users'
import { computed } from 'vue'
import { mdiPencil } from '@mdi/js'

const props = defineProps<{
  user: User
}>()

const createdAt = computed(() => {
  return formatPrettyDate(props.user.created_at)
})

const emailVerifiedAt = computed(() => {
  if (!props.user.email_verified_at) return 'not-verified'

  return formatPrettyDate(props.user.email_verified_at)
})

const lastActiveAt = computed(() => {
  if (!props.user.last_session) return 'no data'

  return formatPrettyDate(props.user.last_session.updated_at)
})
</script>
<template>
  <tr>
    <td>{{ user.email }}</td>
    <td>{{ user.secret ? 'yes' : 'no' }}</td>
    <td>{{ user.role }}</td>
    <td>{{ emailVerifiedAt }}</td>
    <td>{{ createdAt }}</td>
    <td>{{ lastActiveAt }}</td>
    <td class="text-right">
      <BaseButton
        :to="{
          name: 'admin-users-single',
          params: {
            id: user.id
          }
        }"
        :icon="mdiPencil"
      />
    </td>
  </tr>
</template>
