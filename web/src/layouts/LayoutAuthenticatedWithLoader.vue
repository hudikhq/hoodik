<script setup lang="ts">
import { ref } from 'vue'
import LayoutAuthenticated from './LayoutAuthenticated.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'

const puppyLoader = ref(true)
</script>

<template>
  <Suspense>
    <LayoutAuthenticated v-slot="{ authenticated, keypair }">
      <slot :authenticated="authenticated" :keypair="keypair" />
    </LayoutAuthenticated>

    <template #fallback>
      <div
        class="flex min-h-screen items-center justify-center bg-gradient-to-tr from-brownish-700 via-brownish-900 to-brownish-800"
      >
        <PuppyLoader v-model="puppyLoader" />
      </div>
    </template>
  </Suspense>
</template>
