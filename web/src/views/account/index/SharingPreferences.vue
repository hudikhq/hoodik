<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import CardBox from '@/components/ui/CardBox.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiEmailFastOutline } from '@mdi/js'

import { api as sharesApi } from '!/shares'
import { store as loginStore } from '!/auth/login'
import { errorNotification, notification } from '!/index'

import type { User } from 'types'

const props = defineProps<{
  user: User
  class?: string
}>()

const login = loginStore()

const enabled = ref<boolean>(props.user.share_notifications_enabled ?? true)
const saving = ref(false)

watch(
  () => props.user.share_notifications_enabled,
  (next) => {
    enabled.value = next ?? true
  }
)

async function toggle(): Promise<void> {
  saving.value = true
  const desired = !enabled.value
  try {
    const updated = await sharesApi.patchMe({ share_notifications_enabled: desired })
    enabled.value = updated.share_notifications_enabled
    const auth = login.authenticated
    if (auth) {
      login.set({
        ...auth,
        user: { ...auth.user, share_notifications_enabled: enabled.value }
      })
    }
    notification(
      'Sharing notifications updated',
      enabled.value
        ? 'You will receive an email when someone shares a file with you.'
        : 'You will no longer receive sharing emails.',
      'success'
    )
  } catch (err) {
    errorNotification(err)
  } finally {
    saving.value = false
  }
}

const label = computed(() =>
  enabled.value ? 'You will receive sharing emails.' : 'Sharing emails are off.'
)
</script>

<template>
  <CardBox :class="props.class">
    <div class="flex items-center gap-2 mb-4">
      <BaseIcon :path="mdiEmailFastOutline" :size="14" class="text-brownish-400 dark:text-brownish-100" />
      <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100">
        Sharing notifications
      </p>
    </div>

    <label class="flex items-start gap-3 cursor-pointer">
      <input
        type="checkbox"
        :checked="enabled"
        :disabled="saving"
        class="mt-1"
        data-testid="account-share-notifications-toggle"
        @change="toggle"
      />
      <span>
        <span class="text-sm font-medium">Email me when someone shares a file with me</span>
        <span class="block text-xs text-brownish-400 mt-1" data-testid="account-share-notifications-label">
          {{ label }}
        </span>
      </span>
    </label>
  </CardBox>
</template>
