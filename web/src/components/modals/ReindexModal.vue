<script setup lang="ts">
/**
 * Shown once per account after the search re-key, while the client rebuilds
 * its own index.
 *
 * The work has to happen here rather than on the server: the new index stores
 * tags keyed on material the server never sees, and note bodies have to be
 * downloaded and decrypted to be re-indexed at all. Until a file is done it
 * simply does not turn up in search, so the user gets told what is happening
 * rather than left wondering why their files vanished from the search box.
 */
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtons from '@/components/ui/BaseButtons.vue'
import CardBox from '@/components/ui/CardBox.vue'
import OverlayLayer from '@/components/ui/OverlayLayer.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'

import { store as reindexStore } from '!/storage/reindex'
import { store as loginStore } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'

const { t } = useI18n()
const reindex = reindexStore()
const login = loginStore()
const crypto = cryptoStore()

const counts = computed(() => ({ done: reindex.done, total: reindex.total }))

/**
 * Start once the session and the unlocked private key are both in place — the
 * sweep needs the key to derive tags, and re-index writes are owner-scoped.
 * Nothing is persisted about having run: the server reports what is still
 * pending, so a cancelled or interrupted sweep simply resumes here next time.
 */
async function start() {
  if (!login.authenticated?.user?.id) return
  if (!crypto.keypair?.input && !crypto.keypair?.wrappingPrivate) return
  if (reindex.running) return

  if ((await reindex.countPending()) === 0) return

  await reindex.run(crypto.keypair)
}

watch(
  () => [login.authenticated?.user?.id, crypto.keypair?.input, crypto.keypair?.wrappingPrivate],
  start,
  { immediate: true }
)
</script>

<template>
  <OverlayLayer v-show="reindex.visible" z-index="z-50">
    <CardBox class="shadow-lg max-h-modal w-11/12 md:w-3/5 lg:w-2/5 xl:w-4/12 z-50">
      <CardBoxComponentTitle :title="t('reindex.title')" />

      <div class="space-y-4">
        <p class="text-sm text-gray-600 dark:text-gray-300">
          {{ t('reindex.explanation') }}
        </p>

        <div>
          <div
            class="h-2 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700"
            role="progressbar"
            :aria-valuenow="reindex.progress"
            aria-valuemin="0"
            aria-valuemax="100"
            :aria-label="t('reindex.title')"
          >
            <div
              class="h-full rounded-full bg-blue-600 transition-all duration-300"
              :style="{ width: `${reindex.progress}%` }"
            />
          </div>

          <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
            {{ t('reindex.progress', counts) }}
          </p>
        </div>

        <p v-if="reindex.failed > 0" class="text-xs text-yellow-600 dark:text-yellow-400">
          {{ t('reindex.failed', { count: reindex.failed }) }}
        </p>
      </div>

      <template #footer>
        <BaseButtons>
          <BaseButton
            :label="t('reindex.background')"
            color="info"
            @click="reindex.continueInBackground()"
          />
          <BaseButton
            :label="t('reindex.cancel')"
            color="light"
            @click="reindex.cancel()"
          />
        </BaseButtons>
      </template>
    </CardBox>
  </OverlayLayer>
</template>
