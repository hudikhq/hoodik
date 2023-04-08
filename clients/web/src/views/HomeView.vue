<script setup lang="ts">
import LayoutAuthenticated from '@/layouts/LayoutAuthenticated.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import SectionTitleLineWithButton from '@/components/ui/SectionTitleLineWithButton.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { mdiChartTimelineVariant, mdiGithub } from '@mdi/js'
</script>

<template>
  <Suspense>
    <LayoutAuthenticated>
      <SectionMain>
        <SectionTitleLineWithButton :icon="mdiChartTimelineVariant" title="Overview" main>
          <BaseButton
            href="https://github.com/htunlogic/hoodik"
            target="_blank"
            :icon="mdiGithub"
            label="Star on GitHub"
            color="contrast"
            rounded-full
            small
          />
        </SectionTitleLineWithButton>
      </SectionMain>
    </LayoutAuthenticated>
  </Suspense>
</template>
