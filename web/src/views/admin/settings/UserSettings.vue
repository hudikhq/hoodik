<script lang="ts" setup>
import CardBox from '@/components/ui/CardBox.vue'
import UniversalCheckbox from '@/components/ui/UniversalCheckbox.vue'
import type { Data } from 'types/admin/settings'
import ListInput from '@/components/ui/ListInput.vue'
import { computed } from 'vue'
import { formatSize } from '!/index'
import SliderInput from '@/components/ui/SliderInput.vue'

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

const displayQuota = computed(() => {
  return formatSize(data.value?.users.quota_bytes || 0)
})

const unlimitedQuota = computed({
  get() {
    return typeof data.value?.users.quota_bytes !== 'number'
  },
  set(value) {
    const d = data.value

    if (!d) {
      return
    }

    if (value) {
      d.users.quota_bytes = undefined
    } else {
      d.users.quota_bytes = 0
    }

    data.value = d
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
    <UniversalCheckbox
      label="Allow users to use unlimited storage"
      name="unlimited_quota"
      v-model="unlimitedQuota"
      :disabled="loading"
    />

    <div class="flex flex-col mb-4" v-if="!unlimitedQuota">
      <div class="">
        <SliderInput v-model="data.users.quota_bytes" :max="1024 * 1024 * 1024 * 1024" />
      </div>
      <div class="flex w-2/12">
        {{ displayQuota }}
      </div>
    </div>
  </CardBox>
</template>
