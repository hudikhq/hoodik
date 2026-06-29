<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { RouterLink, RouterView } from 'vue-router'

import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import { useCapability } from '@/composables/useCapability'
import { store as sharesStoreFactory, capabilitiesStore } from '!/shares'

const { sharingEnabled } = useCapability()
const caps = capabilitiesStore()
const shares = sharesStoreFactory()

const unreadCount = computed(() => shares.unreadCount)

interface SubTab {
  name: string
  to: { name: string }
  label: string
  testid: string
  hidden?: boolean
}

const subTabs = computed<SubTab[]>(() => [
  { name: 'share-public', to: { name: 'share-public' }, label: 'Public links', testid: 'share-hub-tab-public' },
  {
    name: 'share-activity',
    to: { name: 'share-activity' },
    label: 'Activity',
    testid: 'share-hub-tab-activity',
    hidden: !sharingEnabled.value || !caps.auditLog
  },
  {
    name: 'share-groups',
    to: { name: 'share-groups' },
    label: 'Groups',
    testid: 'share-hub-tab-groups',
    hidden: !sharingEnabled.value || !caps.shareGroups
  }
])

const visibleSubTabs = computed(() => subTabs.value.filter((tab) => !tab.hidden))

async function refreshIncoming(): Promise<void> {
  if (!sharingEnabled.value) return
  try {
    await shares.loadIncoming(50, 0)
  } catch {
    // The store has surfaced the error; the UI keeps rendering whatever
    // cached rows it already has.
  }
}

onMounted(() => {
  refreshIncoming()
})
</script>

<template>
  <LayoutAuthenticatedWithLoader v-slot="{ authenticated, keypair }">
    <SectionMain v-if="authenticated">
      <div class="flex flex-col gap-4">
        <header class="flex items-center justify-between gap-3">
          <h1 class="text-xl sm:text-2xl font-semibold">Share</h1>
          <div
            v-if="unreadCount > 0"
            class="text-xs px-2.5 py-1 rounded-full bg-redish-500 text-white font-medium"
            data-testid="share-hub-unread-badge"
          >
            {{ unreadCount }} new
          </div>
        </header>

        <nav
          class="flex items-center gap-1 sm:gap-3 border-b border-brownish-200 dark:border-brownish-700 overflow-x-auto scrollbar-hide"
          data-testid="share-hub-subtabs"
        >
          <RouterLink
            v-for="tab in visibleSubTabs"
            :key="tab.name"
            :to="tab.to"
            :data-testid="tab.testid"
            class="min-h-11 inline-flex items-center px-3 sm:px-4 py-2 text-sm border-b-2 -mb-px whitespace-nowrap transition-colors"
            active-class="border-redish-500 text-redish-500 dark:text-redish-200 font-medium"
            exact-active-class="border-redish-500 text-redish-500 dark:text-redish-200 font-medium"
            :class="'border-transparent text-brownish-400 dark:text-brownish-300 hover:text-brownish-700 dark:hover:text-brownish-100'"
          >
            {{ tab.label }}
          </RouterLink>
        </nav>

        <RouterView v-slot="{ Component }">
          <component
            :is="Component"
            :authenticated="authenticated"
            :keypair="keypair"
          />
        </RouterView>
      </div>
    </SectionMain>
  </LayoutAuthenticatedWithLoader>
</template>
