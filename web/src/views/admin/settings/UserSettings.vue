<script lang="ts" setup>
import CardBox from '@/components/ui/CardBox.vue'
import UniversalCheckbox from '@/components/ui/UniversalCheckbox.vue'
import type { Data } from 'types/admin/settings'
import ListInput from '@/components/ui/ListInput.vue'
import { computed } from 'vue'
import QuotaSlider from '@/components/ui/QuotaSlider.vue'

const props = defineProps<{
  modelValue?: Data
  loading: boolean
}>()

const emits = defineEmits(['update:modelValue'])

const data = computed({
  get() {
    return props.modelValue
  },
  set(value) {
    emits('update:modelValue', value)
  }
})
</script>
<template>
  <CardBox v-if="data">
    <h1 class="text-2xl text-white mb-4">User settings</h1>
    <UniversalCheckbox
      label="Allow registration to new users"
      name="allow_register"
      v-model="data.users.allow_register"
      :disabled="loading"
    />
    <span class="text-sm text-brownish-300">
      If the registration is turned off, users will be able to register only when they are invited,
      or if their email matches any of the whitelist (and not blacklist) rules below
    </span>

    <div class="mt-4">
      <ListInput v-model="data.users.email_whitelist" label="Whitelist" :disabled="loading" />
    </div>
    <span class="text-sm text-brownish-300">
      Add patterns that will be used to validate the email address of the user. You can use asterisk
      (*) to create a pattern. <br />
      Examples:

      <ul>
        <li>
          <strong>*@example.com</strong> - only emails ending in *@example.com are allowed
          (someone@example.com)
        </li>
        <li>
          <strong>someone@example.*</strong> - only emails starting with the rule can register
          (someone@example.org)
        </li>
        <li>
          <strong>*@example.*</strong> - only emails containing the given rule can register
          (someone@example.org, anyone@example.com, etc.)
        </li>
      </ul>
    </span>

    <div class="mt-4">
      <ListInput v-model="data.users.email_blacklist" label="Blacklist" :disabled="loading" />
    </div>
    <span class="text-sm text-brownish-300">
      Similarly to whitelist, just the opposite. User emails matching this patterns won't be allowed
      to register, even with an invitation. <br />
      Examples:

      <ul>
        <li>
          <strong>*@example.com</strong> - only emails ending in *@example.com are allowed
          (someone@example.com)
        </li>
        <li>
          <strong>someone@example.*</strong> - only emails starting with the rule can register
          (someone@example.org)
        </li>
        <li>
          <strong>*@example.*</strong> - only emails containing the given rule can register
          (someone@example.org, anyone@example.com, etc.)
        </li>
      </ul>
    </span>

    <h3 class="text-lg mt-4">Storage quota for users</h3>
    <QuotaSlider
      v-model="data.users.quota_bytes"
      :disabled="loading"
      title="Default quota for new users"
    />
  </CardBox>
</template>
