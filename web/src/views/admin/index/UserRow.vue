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
    <td :data-label="$t('common.email')">{{ user.email }}</td>
    <td :data-label="$t('admin.users.tfa')">
      <span v-if="user.secret" class="text-greeny-500 dark:text-greeny-300">{{ $t('admin.users.tfaEnabled') }}</span>
      <span v-else class="text-brownish-400 dark:text-brownish-50">{{ $t('admin.users.tfaOff') }}</span>
    </td>
    <td :data-label="$t('admin.role')">
      <span v-if="user.role" class="text-xs font-medium text-orangy-800 dark:text-orangy-400">{{ user.role }}</span>
      <span v-else class="text-brownish-400 dark:text-brownish-50">—</span>
    </td>
    <td :data-label="$t('admin.users.emailActivated')">
      <span v-if="emailVerifiedAt" class="text-greeny-500 dark:text-greeny-300">{{ emailVerifiedAt }}</span>
      <span v-else class="text-redish-500 dark:text-redish-100">{{ $t('admin.users.unverified') }}</span>
    </td>
    <td :data-label="$t('common.created')">{{ createdAt }}</td>
    <td :data-label="$t('admin.users.lastActive')">
      <span v-if="lastActiveAt">{{ lastActiveAt }}</span>
      <span v-else class="text-brownish-400 dark:text-brownish-50">{{ $t('common.never') }}</span>
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
