<script lang="ts" setup>
import type { WhitelistOrBlacklist } from 'types/admin/settings'
import { computed, ref } from 'vue'
import BaseButton from './BaseButton.vue'
import { mdiPlus, mdiMinus } from '@mdi/js'

const add = ref('')

const props = defineProps<{
  modelValue?: WhitelistOrBlacklist
  label?: string
  disabled?: boolean
}>()

const emits = defineEmits(['update:modelValue'])

const model = computed({
  get() {
    return props.modelValue || { rules: [] }
  },
  set(value) {
    emits('update:modelValue', value)
  }
})

const insert = () => {
  if (!add.value) {
    return
  }

  const m = model.value

  if (!m.rules.some((rule) => rule === add.value)) {
    m.rules.push(add.value)
    model.value = m
  }

  add.value = ''
}

const remove = (index: number) => {
  const m = model.value

  m.rules = model.value.rules.filter((_, i) => i !== index)

  model.value = m
}

const inputClass =
  'h-[34px] w-auto mt-2 px-3 text-lg rounded-md transition duration-150 ease-in-out ' +
  'bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-600 ' +
  'text-brownish-900 dark:text-white placeholder-brownish-100/60 dark:placeholder-brownish-400 ' +
  'disabled:opacity-80 focus:outline-none focus:ring-2 focus:ring-offset-0 ' +
  'focus:ring-redish-400/60 dark:focus:ring-redish-500/50'
</script>
<template>
  <span class="sm:block" for="rules" v-if="label"> {{ label }} </span>

  <div v-for="(item, index) in model.rules" :key="index">
    <div>
      <input v-model="model.rules[index]" :disabled="true" type="text" :class="inputClass" />
      <BaseButton
        color="danger"
        :disabled="disabled"
        @click="remove(index)"
        :icon="mdiMinus"
        :small="true"
        class="ml-2 h-[34px]"
      />
    </div>
  </div>

  <form @submit.prevent="insert">
    <input
      v-model="add"
      :disabled="disabled"
      type="text"
      :class="inputClass"
      placeholder="*@example.com"
    />
    <BaseButton
      type="submit"
      color="info"
      :disabled="disabled"
      :icon="mdiPlus"
      :small="true"
      class="ml-2 h-[34px]"
    />
  </form>
</template>
