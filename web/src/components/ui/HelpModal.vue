<script setup lang="ts">
import { computed } from 'vue'
import { mdiOpenInNew } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'

const value = defineModel<boolean>({ required: true })

/**
 * Chord labels follow the platform the browser is actually running on —
 * printing Ctrl to a Mac user is worse than printing nothing.
 */
const isApple = computed(
  () => typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform)
)
const mod = computed(() => (isApple.value ? '⌘' : 'Ctrl'))
const shift = computed(() => (isApple.value ? '⇧' : 'Shift'))

const appShortcuts = computed(() => [
  { keys: [`${mod.value}K`], label: 'help.shortcuts.search' },
  { keys: [`${mod.value}/`], label: 'help.shortcuts.help' }
])

const editorShortcuts = computed(() => [
  { keys: [`${mod.value}B`], label: 'help.shortcuts.bold' },
  { keys: [`${mod.value}I`], label: 'help.shortcuts.italic' },
  { keys: [`${mod.value}K`], label: 'help.shortcuts.link' },
  { keys: [`${mod.value}S`], label: 'help.shortcuts.save' },
  { keys: [`${mod.value}Z`, `${shift.value}${mod.value}Z`], label: 'help.shortcuts.undoRedo' }
])

// Anyone reading this is already on a running server — either their own or one
// they pay for — so a self-hosting guide has no audience here.
const links = [
  { href: 'https://github.com/hudikhq/hoodik', label: 'help.links.source' },
  { href: 'https://hoodik.io/apps?utm_source=hoodik-server', label: 'help.links.apps' }
]
</script>

<template>
  <CardBoxModal v-model="value" :title="$t('help.title')" has-cancel hide-submit>
    <!-- Nothing here is being cancelled — the panel only reads. -->
    <template #buttons>
      <BaseButton :label="$t('common.close')" color="light" @click="value = false" />
    </template>

    <div class="space-y-6 text-sm">
      <section>
        <h3 class="text-xs font-semibold text-brownish-400 dark:text-brownish-50 mb-2">
          {{ $t('help.shortcuts.title') }}
        </h3>
        <dl class="space-y-1.5">
          <div v-for="s in appShortcuts" :key="s.label" class="flex items-baseline justify-between gap-4">
            <dt>{{ $t(s.label) }}</dt>
            <dd class="flex gap-1 shrink-0">
              <kbd
                v-for="k in s.keys"
                :key="k"
                class="px-1.5 py-0.5 rounded border border-paper-300 dark:border-brownish-600 bg-paper-100 dark:bg-brownish-900 font-mono text-xs"
                >{{ k }}</kbd
              >
            </dd>
          </div>
        </dl>

        <h4 class="text-xs font-medium text-brownish-400 dark:text-brownish-50 mt-4 mb-2">
          {{ $t('help.shortcuts.inNotes') }}
        </h4>
        <dl class="space-y-1.5">
          <div v-for="s in editorShortcuts" :key="s.label" class="flex items-baseline justify-between gap-4">
            <dt>{{ $t(s.label) }}</dt>
            <dd class="flex gap-1 shrink-0">
              <kbd
                v-for="k in s.keys"
                :key="k"
                class="px-1.5 py-0.5 rounded border border-paper-300 dark:border-brownish-600 bg-paper-100 dark:bg-brownish-900 font-mono text-xs"
                >{{ k }}</kbd
              >
            </dd>
          </div>
        </dl>
      </section>

      <section>
        <h3 class="text-xs font-semibold text-brownish-400 dark:text-brownish-50 mb-2">
          {{ $t('help.facts.title') }}
        </h3>
        <ul class="space-y-2 text-brownish-600 dark:text-brownish-50">
          <li>{{ $t('help.facts.recoveryKey') }}</li>
          <li>{{ $t('help.facts.sharing') }}</li>
          <li>{{ $t('help.facts.server') }}</li>
        </ul>
      </section>

      <section>
        <h3 class="text-xs font-semibold text-brownish-400 dark:text-brownish-50 mb-2">
          {{ $t('help.links.title') }}
        </h3>
        <ul class="space-y-1.5">
          <li v-for="l in links" :key="l.href">
            <a
              :href="l.href"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex items-center gap-1 text-redish-700 dark:text-redish-100 hover:underline"
            >
              {{ $t(l.label) }}
              <BaseIcon :path="mdiOpenInNew" :size="13" aria-hidden="true" />
            </a>
          </li>
        </ul>
      </section>
    </div>
  </CardBoxModal>
</template>
