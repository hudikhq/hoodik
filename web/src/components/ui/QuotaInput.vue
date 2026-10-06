<script setup lang="ts">
import UniversalCheckbox from './UniversalCheckbox.vue'
import { computed, ref, watch } from 'vue'

const props = defineProps<{
  modelValue: number | undefined
  disabled?: boolean
  title?: string
}>()

const emits = defineEmits(['update:modelValue'])

const UNITS = {
  MB: 1024 ** 2,
  GB: 1024 ** 3,
  TB: 1024 ** 4
} as const
type Unit = keyof typeof UNITS

const amount = ref<number | undefined>()
const unit = ref<Unit>('GB')

function toBytes(value: number | undefined, u: Unit): number {
  return Math.round((value || 0) * UNITS[u])
}

function syncFromModel(bytes: number | undefined, force = false) {
  if (typeof bytes !== 'number') {
    return
  }
  // Don't reformat what the user is typing (e.g. "1.50" or an emptied field would get rewritten)
  if (!force && toBytes(amount.value, unit.value) === bytes) {
    return
  }
  if (bytes >= UNITS.TB) {
    unit.value = 'TB'
  } else if (bytes >= UNITS.GB || bytes === 0) {
    unit.value = 'GB'
  } else {
    unit.value = 'MB'
  }
  amount.value = Math.round((bytes / UNITS[unit.value]) * 100) / 100
}

syncFromModel(props.modelValue, true)
watch(
  () => props.modelValue,
  (bytes) => syncFromModel(bytes)
)

function emitBytes() {
  emits('update:modelValue', toBytes(amount.value, unit.value))
}

function onAmountInput(event: Event) {
  const value = (event.target as HTMLInputElement).valueAsNumber
  amount.value = Number.isFinite(value) ? Math.max(0, value) : undefined
  emitBytes()
}

const defaultQuota = computed({
  get() {
    return typeof props.modelValue !== 'number'
  },
  set(value) {
    emits('update:modelValue', value ? undefined : toBytes(amount.value, unit.value))
  }
})

const inputDisabled = computed(() => props.disabled || defaultQuota.value)
</script>
<template>
  <h3 class="text-lg mt-4" v-if="title">{{ title }}</h3>
  <div class="flex w-full items-center gap-3">
    <UniversalCheckbox
      :label="$t('ui.quotaInput.default')"
      name="default_quota"
      v-model="defaultQuota"
      :disabled="disabled"
    />
    <div class="flex flex-1 gap-2">
      <input
        type="number"
        min="0"
        step="any"
        :value="amount"
        @input="onAmountInput"
        :disabled="inputDisabled"
        :aria-label="title || $t('admin.user.storageQuota')"
        data-testid="quota-amount-input"
        class="w-full min-w-0 bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg px-3 py-2 focus:outline-none focus:border-redish-500 disabled:opacity-50"
      />
      <select
        v-model="unit"
        @change="emitBytes"
        :disabled="inputDisabled"
        data-testid="quota-unit-select"
        class="shrink-0 bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-700 text-sm rounded-lg pl-3 pr-9 py-2 focus:outline-none focus:border-redish-500 disabled:opacity-50"
      >
        <option v-for="u in Object.keys(UNITS)" :key="u" :value="u">{{ $t(`size.${u}`) }}</option>
      </select>
    </div>
  </div>
</template>
