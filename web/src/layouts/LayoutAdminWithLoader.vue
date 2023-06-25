<script setup lang="ts">
import { ref } from 'vue'
import LayoutAuthenticated from './LayoutAuthenticated.vue'
import LayoutAuthenticatedClear from './LayoutAuthenticatedClear.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'

const props = defineProps<{
  clear?: boolean
}>()

const puppyLoader = ref(true)
</script>

<template>
  <Suspense>
    <LayoutAuthenticated v-if="!props.clear" v-slot="{ authenticated, keypair }">
      <slot :authenticated="authenticated" :keypair="keypair" />
    </LayoutAuthenticated>

    <LayoutAuthenticatedClear v-else v-slot="{ authenticated, keypair }">
      <slot :authenticated="authenticated" :keypair="keypair" />
    </LayoutAuthenticatedClear>

    <template #fallback>
      <div
        class="flex min-h-screen items-center justify-center bg-gradient-to-tr from-brownish-700 via-brownish-900 to-brownish-800"
      >
        <PuppyLoader v-model="puppyLoader" />
      </div>
    </template>
  </Suspense>
</template>
