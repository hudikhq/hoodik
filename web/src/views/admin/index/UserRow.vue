<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButton from '@/components/ui/BaseButton.vue'
import type { User } from 'types/admin/users'
import { computed } from 'vue'
import { mdiPencil } from '@mdi/js'

const props = defineProps<{
  user: User
}>()

const createdAt = computed(() => formatPrettyDate(props.user.created_at))

const emailVerifiedAt = computed(() => {
  if (!props.user.email_verified_at) return null
  return formatPrettyDate(props.user.email_verified_at)
})

const lastActiveAt = computed(() => {
  if (!props.user.last_session) return null
  return formatPrettyDate(props.user.last_session.updated_at)
})
</script>
<template>
  <tr>
    <td data-label="Email">{{ user.email }}</td>
    <td data-label="TFA">
      <span v-if="user.secret" class="text-greeny-500 dark:text-greeny-400">enabled</span>
      <span v-else class="text-brownish-400">off</span>
    </td>
    <td data-label="Role">
      <span v-if="user.role" class="text-xs font-medium uppercase tracking-wider text-orangy-400">{{ user.role }}</span>
      <span v-else class="text-brownish-400">—</span>
    </td>
    <td data-label="Email Activated">
      <span v-if="emailVerifiedAt" class="text-greeny-500 dark:text-greeny-400">{{ emailVerifiedAt }}</span>
      <span v-else class="text-redish-500">unverified</span>
    </td>
    <td data-label="Created">{{ createdAt }}</td>
    <td data-label="Last Active">
      <span v-if="lastActiveAt">{{ lastActiveAt }}</span>
      <span v-else class="text-brownish-400">never</span>
    </td>
    <td class="text-right">
      <BaseButton
        :to="{ name: 'manage-users-single', params: { id: user.id } }"
        :icon="mdiPencil"
        :small="true"
      />
    </td>
  </tr>
</template>
