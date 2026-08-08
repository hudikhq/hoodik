<script setup lang="ts">
import { store as style } from '!/style'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import { ref } from 'vue'

const styleStore = style()
const puppyLoader = ref(true)
</script>

<template>
  <div :class="{ dark: styleStore.darkMode }">
    <div class="bg-paper-50 text-brownish-900 dark:bg-brownish-800 dark:text-dirty-white">
      <Suspense>
        <slot />

        <template #fallback>
          <div
            class="flex min-h-screen items-center justify-center bg-gradient-to-tr from-paper-100 via-paper-50 to-paper-100 dark:from-brownish-700 dark:via-brownish-900 dark:to-brownish-800"
          >
            <PuppyLoader v-model="puppyLoader" />
          </div>
        </template>
      </Suspense>
    </div>
  </div>
</template>
