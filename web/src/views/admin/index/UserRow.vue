<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButton from '@/components/ui/BaseButton.vue'
import type { User } from 'types/admin/users'
import { computed } from 'vue'
import { mdiPencil } from '@mdi/js'

const props = defineProps<{
  user: User
}>()

const user = computed(() => {
  return props.user
})

const createdAt = computed(() => {
  return formatPrettyDate(user.value.created_at)
})

const emailVerifiedAt = computed(() => {
  if (!user.value.email_verified_at) return 'not-verified'

  return formatPrettyDate(user.value.email_verified_at)
})

const lastActiveAt = computed(() => {
  if (!user.value.last_session) return 'no data'

  return formatPrettyDate(user.value.last_session.updated_at)
})
</script>
<template>
  <tr>
    <td>{{ user.email }}</td>
    <td>{{ user.secret ? 'yes' : 'no' }}</td>
    <td>
      {{ user.role ? user.role : 'n/a' }}
    </td>
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
