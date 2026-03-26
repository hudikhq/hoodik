<script setup lang="ts">
import { formatPrettyDate } from '!/index'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import { mdiShieldOffOutline } from '@mdi/js'
import type { Authenticated, Session } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  authenticated: Authenticated
  session: Session
}>()

const emits = defineEmits(['revoke'])

const createdAt = computed(() => formatPrettyDate(props.session.created_at))
const updatedAt = computed(() => formatPrettyDate(props.session.updated_at))
const expiresAt = computed(() => formatPrettyDate(props.session.expires_at))

const expired = computed(() => props.session.expires_at < new Date().valueOf() / 1000)
const isCurrent = computed(() => props.authenticated.session.id === props.session.id)
const isActive = computed(() => !!props.session.refresh && !expired.value)
</script>
<template>
  <tr :class="{ 'opacity-50': !isActive && !isCurrent }">
    <td data-label="IP Address">{{ session.ip }}</td>
    <td data-label="Device" class="max-w-[200px] truncate text-sm">{{ session.user_agent }}</td>
    <td data-label="Signed in">{{ createdAt }}</td>
    <td data-label="Last seen">{{ updatedAt }}</td>
    <td data-label="Expires">
      <span v-if="isCurrent">—</span>
      <span v-else-if="expired" class="text-brownish-400">expired</span>
      <span v-else>{{ expiresAt }}</span>
    </td>
    <td data-label="Status">
      <span v-if="isCurrent" class="inline-flex items-center text-xs font-medium bg-greeny-500/15 text-greeny-500 dark:text-greeny-400 px-2 py-0.5 rounded-full">current</span>
      <span v-else-if="isActive" class="inline-flex items-center text-xs font-medium bg-blueish-500/15 text-blueish-400 px-2 py-0.5 rounded-full">active</span>
      <span v-else-if="expired" class="inline-flex items-center text-xs font-medium bg-brownish-100 dark:bg-brownish-700 text-brownish-400 px-2 py-0.5 rounded-full">expired</span>
      <span v-else class="inline-flex items-center text-xs font-medium bg-redish-500/15 text-redish-500 dark:text-redish-400 px-2 py-0.5 rounded-full">revoked</span>
    </td>
    <td>
      <BaseButtonConfirm
        :icon="mdiShieldOffOutline"
        @confirm="emits('revoke', session)"
        label="Revoke"
        confirm-label="Yes, revoke"
        color="danger"
        :small="true"
        :disabled="!session.refresh || isCurrent || expired"
      />
    </td>
  </tr>
</template>
