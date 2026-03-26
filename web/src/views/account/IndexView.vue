<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import MyDetails from './index/MyDetails.vue'
import StorageStats from './index/StorageStats.vue'
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
        <div class="flex flex-col sm:flex-row gap-4">
          <MyDetails
            class="w-full sm:w-1/2"
            :user="authenticated.user"
            @disable-tfa="disableTfaModal = true"
            @enable-tfa="enableTfaModal = true"
          />
          <StorageStats class="w-full sm:w-1/2" />
        </div>

        <div class="mt-4">
          <ActivityInner class="w-full" />
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAuthenticatedWithLoader>
</template>
