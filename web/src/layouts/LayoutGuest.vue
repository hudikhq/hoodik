<script setup lang="ts">
import { store as style } from '!/style'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import { ref } from 'vue'

const styleStore = style()
const puppyLoader = ref(true)
</script>

<template>
  <div :class="{ dark: styleStore.darkMode }">
    <div class="bg-brownish-50 dark:bg-brownish-800 dark:text-brownish-100">
      <Suspense>
        <slot />

        <template #fallback>
          <div
            class="flex min-h-screen items-center justify-center bg-gradient-to-tr from-brownish-700 via-brownish-900 to-brownish-800"
          >
            <PuppyLoader v-model="puppyLoader" />
          </div>
        </template>
      </Suspense>
    </div>
  </div>
</template>
