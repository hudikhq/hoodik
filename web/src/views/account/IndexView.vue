<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import MyDetails from './index/MyDetails.vue'
import StorageStats from './index/StorageStats.vue'
import EnableTfaModal from '@/components/modals/EnableTfaModal.vue'
import DisableTfaModal from '@/components/modals/DisableTfaModal.vue'
import { ref } from 'vue'

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
        <div class="flex space-x-2">
          <MyDetails
            :user="authenticated.user"
            @disable-tfa="disableTfaModal = true"
            @enable-tfa="enableTfaModal = true"
          />
          <StorageStats />
        </div>
      </SectionMain>
    </Suspense>
  </LayoutAuthenticatedWithLoader>
</template>
