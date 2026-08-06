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
    <td :data-label="$t('common.email')">{{ session.email }}</td>
    <td :data-label="$t('admin.sessions.ipAddress')">{{ session.ip }}</td>
    <td :data-label="$t('admin.sessions.device')" class="max-w-[200px] truncate text-sm">{{ session.user_agent }}</td>
    <td :data-label="$t('admin.sessions.signedIn')">{{ createdAt }}</td>
    <td :data-label="$t('admin.sessions.lastSeen')">{{ updatedAt }}</td>
    <td :data-label="$t('admin.expires')">{{ expiresAt }}</td>
    <td :data-label="$t('admin.status')">
      <span v-if="session.active" class="inline-flex items-center text-xs font-medium bg-blueish-500/15 text-blueish-400 px-2 py-0.5 rounded-full">{{ $t('admin.sessions.active') }}</span>
      <span v-else class="inline-flex items-center text-xs font-medium bg-paper-100 dark:bg-brownish-700 text-brownish-400 px-2 py-0.5 rounded-full">{{ $t('admin.sessions.revoked') }}</span>
    </td>
    <td>
      <BaseButtonConfirm
        :icon="mdiShieldOffOutline"
        @confirm="emits('kill', session)"
        :label="$t('admin.revoke')"
        :confirm-label="$t('admin.confirmRevoke')"
        color="danger"
        :small="true"
        :disabled="!session.active"
      />
    </td>
  </tr>
</template>
