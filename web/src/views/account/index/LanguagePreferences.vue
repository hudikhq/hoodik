<script setup lang="ts">
import { ref } from 'vue'

import CardBox from '@/components/ui/CardBox.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiTranslate } from '@mdi/js'

import { api as sharesApi } from '!/shares'
import { SUPPORTED_LOCALES, currentLocale, setLocale, type SupportedLocale } from '@/i18n'

const props = defineProps<{
  class?: string
}>()

const locale = ref<SupportedLocale>(currentLocale())

function change() {
  setLocale(locale.value)

  // Store the preference server-side too, so outbound email (activation,
  // share notifications) follows the user's language. Cosmetic on failure.
  sharesApi.patchMe({ locale: locale.value }).catch(() => undefined)
}
</script>

<template>
  <CardBox :class="props.class">
    <div class="flex items-center gap-2 mb-4">
      <BaseIcon :path="mdiTranslate" :size="14" class="text-brownish-400 dark:text-brownish-100" />
      <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100">
        {{ $t('account.language.title') }}
      </p>
    </div>

    <label class="block">
      <span class="text-sm font-medium">{{ $t('account.language.label') }}</span>
      <select
        v-model="locale"
        data-testid="account-language-select"
        class="mt-2 w-full bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500"
        @change="change"
      >
        <option v-for="(label, code) in SUPPORTED_LOCALES" :key="code" :value="code">
          {{ label }}
        </option>
      </select>
      <span class="block text-xs text-brownish-400 mt-1">
        {{ $t('account.language.description') }}
      </span>
    </label>
  </CardBox>
</template>
