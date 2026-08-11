<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

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
const { t } = useI18n()

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
      t('account.sharing.updatedTitle'),
      enabled.value ? t('account.sharing.enabledText') : t('account.sharing.disabledText'),
      'success'
    )
  } catch (err) {
    errorNotification(err)
  } finally {
    saving.value = false
  }
}

const label = computed(() =>
  enabled.value ? t('account.sharing.statusOn') : t('account.sharing.statusOff')
)
</script>

<template>
  <CardBox :class="props.class">
    <div class="flex items-center gap-2 mb-4">
      <BaseIcon :path="mdiEmailFastOutline" :size="14" class="text-brownish-400 dark:text-brownish-50" />
      <p class="text-xs font-semibold text-brownish-400 dark:text-brownish-50">
        {{ $t('account.sharing.title') }}
      </p>
    </div>

    <label class="checkbox items-start gap-3">
      <input
        type="checkbox"
        :checked="enabled"
        :disabled="saving"
        data-testid="account-share-notifications-toggle"
        @change="toggle"
      />
      <span class="check mt-0.5" />
      <span>
        <span class="text-sm font-medium">{{ $t('account.sharing.toggleLabel') }}</span>
        <span class="block text-xs text-brownish-400 dark:text-brownish-50 mt-1" data-testid="account-share-notifications-label">
          {{ label }}
        </span>
      </span>
    </label>
  </CardBox>
</template>
