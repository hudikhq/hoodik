<script lang="ts" setup>
import CardBox from '@/components/ui/CardBox.vue'
import UniversalCheckbox from '@/components/ui/UniversalCheckbox.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import type { Data } from 'types/admin/settings'
import ListInput from '@/components/ui/ListInput.vue'
import { computed } from 'vue'
import QuotaSlider from '@/components/ui/QuotaSlider.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { mdiContentSave, mdiAccountPlus, mdiEmailSearch, mdiDatabase, mdiShareVariantOutline } from '@mdi/js'

const props = defineProps<{
  modelValue?: Data
  loading: boolean
  error?: string
  class?: string
}>()

const emits = defineEmits(['update:modelValue', 'save'])

const data = computed({
  get() { return props.modelValue },
  set(value) { emits('update:modelValue', value) }
})
</script>
<template>
  <CardBox :class="props.class" v-if="data">
    <div class="-mx-4 -mt-4 px-6 py-5 border-b border-brownish-100 dark:border-brownish-700/50 rounded-t-2xl">
      <div class="flex items-center gap-2">
        <BaseIcon :path="mdiAccountPlus" :size="14" class="text-brownish-400 dark:text-brownish-100" />
        <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100">Registration</p>
      </div>

      <div class="mt-4 space-y-3">
        <div class="p-3 rounded-xl bg-brownish-50/50 dark:bg-brownish-700/20 border border-brownish-100/50 dark:border-brownish-700/30">
          <UniversalCheckbox
            label="Require email verification"
            name="enforce_email_activation"
            v-model="data.users.enforce_email_activation"
            :disabled="loading"
          />
          <p class="text-xs text-brownish-400 dark:text-brownish-100 pl-7 leading-relaxed mt-1">
            New users must click a verification link before they can log in.
          </p>
        </div>

        <div class="p-3 rounded-xl bg-brownish-50/50 dark:bg-brownish-700/20 border border-brownish-100/50 dark:border-brownish-700/30">
          <UniversalCheckbox
            label="Allow public registration"
            name="allow_register"
            v-model="data.users.allow_register"
            :disabled="loading"
          />
          <p class="text-xs text-brownish-400 dark:text-brownish-100 pl-7 leading-relaxed mt-1">
            Anyone can create an account. When off, only invited users or whitelist matches can register.
          </p>
        </div>
      </div>
    </div>

    <div class="-mx-4 px-6 py-5 border-b border-brownish-100 dark:border-brownish-700/50">
      <div class="flex items-center gap-2 mb-4">
        <BaseIcon :path="mdiEmailSearch" :size="14" class="text-brownish-400 dark:text-brownish-100" />
        <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100">Email Filters</p>
      </div>

      <div class="space-y-4">
        <div class="space-y-2">
          <ListInput v-model="data.users.email_whitelist" label="Whitelist" :disabled="loading" />
          <p class="text-xs text-brownish-400 dark:text-brownish-100 leading-relaxed">
            Only emails matching these patterns can register. Use <code class="font-mono bg-brownish-100 dark:bg-brownish-700 px-1 rounded">*@company.com</code> or <code class="font-mono bg-brownish-100 dark:bg-brownish-700 px-1 rounded">*@company.*</code> style globs. Leave empty to allow any email.
          </p>
        </div>

        <div class="space-y-2">
          <ListInput v-model="data.users.email_blacklist" label="Blacklist" :disabled="loading" />
          <p class="text-xs text-brownish-400 dark:text-brownish-100 leading-relaxed">
            Emails matching these patterns are always blocked — overrides whitelist and invitations.
          </p>
        </div>
      </div>
    </div>

    <div class="-mx-4 px-6 py-5 border-b border-brownish-100 dark:border-brownish-700/50">
      <div class="flex items-center gap-2 mb-3">
        <BaseIcon :path="mdiShareVariantOutline" :size="14" class="text-brownish-400 dark:text-brownish-100" />
        <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100">Sharing</p>
      </div>

      <div class="space-y-3">
        <div class="p-3 rounded-xl bg-brownish-50/50 dark:bg-brownish-700/20 border border-brownish-100/50 dark:border-brownish-700/30">
          <UniversalCheckbox
            label="Account-to-account sharing"
            name="sharing_enabled"
            v-model="data.sharing.enabled"
            :disabled="loading"
            data-testid="admin-sharing-enabled-toggle"
          />
          <p class="text-xs text-brownish-400 dark:text-brownish-100 pl-7 leading-relaxed mt-1">
            When off, the Share entry vanishes from every file row and all <code class="font-mono bg-brownish-100 dark:bg-brownish-700 px-1 rounded">/api/shares/*</code> endpoints return 503. Existing share rows are preserved across toggles; nothing is deleted.
          </p>
        </div>

        <div class="p-3 rounded-xl bg-brownish-50/50 dark:bg-brownish-700/20 border border-brownish-100/50 dark:border-brownish-700/30">
          <label class="block text-xs uppercase tracking-wider mb-1 text-brownish-300" for="default_cipher">Default cipher</label>
          <select
            id="default_cipher"
            v-model="data.sharing.default_cipher"
            :disabled="loading"
            data-testid="admin-default-cipher-select"
            class="w-full bg-white dark:bg-brownish-800 border border-brownish-200 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500"
          >
            <option value="aegis128l">AEGIS-128L</option>
            <option value="aegis256">AEGIS-256</option>
            <option value="ascon128a">Ascon-128a</option>
            <option value="chacha20poly1305">ChaCha20-Poly1305</option>
          </select>
          <p class="text-xs text-brownish-400 dark:text-brownish-100 leading-relaxed mt-1">
            Applied to new uploads on every client. Existing files keep the cipher they were encrypted with.
          </p>
        </div>
      </div>
    </div>

    <div class="-mx-4 px-6 py-5 border-b border-brownish-100 dark:border-brownish-700/50">
      <div class="flex items-center gap-2 mb-3">
        <BaseIcon :path="mdiDatabase" :size="14" class="text-brownish-400 dark:text-brownish-100" />
        <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-100">Default Storage Quota</p>
      </div>
      <p class="text-xs text-brownish-400 dark:text-brownish-100 leading-relaxed mb-3">Applied to new users at registration. Existing users keep their current quota.</p>
      <QuotaSlider
        v-model="data.users.quota_bytes"
        :disabled="loading"
      />
    </div>

    <div class="-mx-4 -mb-4 px-6 py-4 rounded-b-2xl">
      <div v-if="error" class="rounded-lg bg-redish-500/10 border border-redish-500/30 px-4 py-3 mb-3">
        <p class="text-sm text-redish-400">{{ error }}</p>
      </div>

      <BaseButton
        color="info"
        :disabled="loading"
        :icon="mdiContentSave"
        :label="loading ? 'Saving…' : 'Save Settings'"
        @click="emits('save')"
      />
    </div>
  </CardBox>
</template>
