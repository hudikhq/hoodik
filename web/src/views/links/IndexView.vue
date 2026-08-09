<script setup lang="ts">
import LayoutLinkBrowser from '@/layouts/LayoutLinkBrowser.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiAlertCircleOutline } from '@mdi/js'
import TableLinks from './index-view/TableLinks.vue'
</script>

<template>
  <LayoutLinkBrowser :clear="false" v-slot="{ Links, on }">
    <SectionMain>
      <div
        v-if="Links.undecryptable"
        role="alert"
        class="flex items-start gap-2 rounded-md border border-orangy-500/30 bg-orangy-500/10 px-3 py-2 mb-3 text-sm text-orangy-700 dark:border-orangy-400/25 dark:bg-orangy-400/10 dark:text-orangy-200"
      >
        <BaseIcon :path="mdiAlertCircleOutline" :size="16" w="w-4" h="h-4" class="mt-0.5 shrink-0" />
        <span>{{ $t('errors.undecryptable') }}</span>
      </div>

      <TableLinks :items="Links.items" :selected="Links.selected" v-on="on" />
    </SectionMain>
  </LayoutLinkBrowser>
</template>
