<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { mdiShieldCheck } from '@mdi/js'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { AppButton } from '@/components/form'
import { store as loginStore } from '!/auth/login'
import * as migrationNotice from '!/auth/migration-notice'

const login = loginStore()
const router = useRouter()
const show = ref(false)

function refresh() {
  const id = login.authenticated?.user?.id
  show.value = !!id && migrationNotice.isPending(id)
}

// The flag is set at the end of the ceremony, on the same tick the session lands,
// so re-check whenever the authenticated user changes.
watch(() => login.authenticated?.user?.id, refresh, { immediate: true })

function acknowledge() {
  const id = login.authenticated?.user?.id
  if (id) migrationNotice.acknowledge(id)
  show.value = false
}

function goToRecoveryKey() {
  acknowledge()
  router.push({ name: 'account' })
}
</script>

<template>
  <CardBoxModal
    v-model="show"
    :title="$t('account.migration.title')"
    button="success"
    :button-label="$t('account.migration.gotIt')"
    @confirm="acknowledge"
  >
    <div class="flex items-start gap-3">
      <BaseIcon :path="mdiShieldCheck" size="32" class="text-greeny-500 shrink-0 mt-1" />
      <div class="space-y-3 text-sm">
        <p>
          {{ $t('account.migration.body1') }}
        </p>
        <i18n-t keypath="account.migration.body2" tag="p" scope="global">
          <template #recoveryKey>
            <strong>{{ $t('account.migration.recoveryKeyTerm') }}</strong>
          </template>
          <template #location>
            <strong>{{ $t('account.migration.recoveryKeyLocation') }}</strong>
          </template>
        </i18n-t>
        <AppButton
          type="button"
          :label="$t('account.migration.getRecoveryKey')"
          color="info"
          @click="goToRecoveryKey"
        />
      </div>
    </div>
  </CardBoxModal>
</template>
