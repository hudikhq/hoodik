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
        class="flex min-h-screen items-center justify-center bg-gradient-to-tr from-paper-100 via-paper-50 to-paper-100 dark:from-brownish-700 dark:via-brownish-900 dark:to-brownish-800"
      >
        <PuppyLoader v-model="puppyLoader" />
      </div>
    </template>
  </Suspense>
</template>
