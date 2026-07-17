<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import MyDetails from './index/MyDetails.vue'
import StorageStats from './index/StorageStats.vue'
import SharingPreferences from './index/SharingPreferences.vue'
import RecoveryKey from './index/RecoveryKey.vue'
import EnableTfaModal from '@/components/modals/EnableTfaModal.vue'
import DisableTfaModal from '@/components/modals/DisableTfaModal.vue'
import { ref } from 'vue'
import ActivityInner from './index/ActivityInner.vue'

const disableTfaModal = ref(false)
const enableTfaModal = ref(false)
</script>
<template>
  <LayoutAuthenticatedWithLoader v-slot="{ authenticated }">
    <EnableTfaModal
      v-if="enableTfaModal && authenticated"
      @confirm="enableTfaModal = false"
      @cancel="enableTfaModal = false"
      v-model="authenticated.user"
    />
    <DisableTfaModal
      v-if="disableTfaModal && authenticated"
      @confirm="disableTfaModal = false"
      @cancel="disableTfaModal = false"
      v-model="authenticated.user"
    />
    <Suspense>
      <SectionMain v-if="authenticated">
        <div class="mb-8">
          <h2 class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100 mb-3 px-1">Profile</h2>
          <div class="flex flex-col lg:flex-row gap-6">
            <MyDetails
              class="w-full lg:w-7/12"
              :user="authenticated.user"
              @disable-tfa="disableTfaModal = true"
              @enable-tfa="enableTfaModal = true"
            />
            <StorageStats class="w-full lg:w-5/12" />
          </div>
        </div>

        <div class="mb-8">
          <h2 class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100 mb-3 px-1">Security</h2>
          <RecoveryKey class="w-full" />
        </div>

        <div class="mb-8">
          <h2 class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100 mb-3 px-1">Sharing</h2>
          <SharingPreferences :user="authenticated.user" class="w-full" />
        </div>

        <div>
          <h2 class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100 mb-3 px-1">Sessions</h2>
          <ActivityInner class="w-full" />
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAuthenticatedWithLoader>
</template>
