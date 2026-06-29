<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiLinkOff } from '@mdi/js'
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass" data-testid="link-unavailable">
        <div class="flex flex-col items-center text-center gap-4 py-4">
          <span
            class="inline-flex items-center justify-center w-16 h-16 rounded-full bg-redish-100/80 text-redish-500 dark:bg-redish-900/40 dark:text-redish-200"
          >
            <BaseIcon :path="mdiLinkOff" :size="32" />
          </span>
          <h1 class="text-2xl font-semibold">Link expired or removed</h1>
          <p class="text-sm text-brownish-500 dark:text-brownish-200 max-w-sm">
            This share link is no longer active. Ask the owner for a new link if you
            still need access to the file.
          </p>
        </div>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
