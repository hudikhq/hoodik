<script lang="ts" setup>
import { format } from 'date-fns'
import { computed, ref, watch } from 'vue'

const props = defineProps<{
  modelValue?: Date | undefined
  name: string
  min?: Date
  disabled?: boolean
}>()

const emit = defineEmits(['update:modelValue'])

const model = computed<Date | undefined>({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v)
})

const date = ref<string>()
const time = ref<string>()

watch(
  () => date.value,
  (v) => {
    let date = undefined

    if (v) {
      date = new Date(v)

      if (time.value) {
        const [hours, minutes] = time.value.split(':')
        date.setHours(parseInt(hours))
        date.setMinutes(parseInt(minutes))
        date.setSeconds(0)
        date.setMilliseconds(0)
      } else {
        date.setHours(0)
        date.setMinutes(0)
        time.value = '00:00'
      }
    }

    model.value = date
  },
  { immediate: true }
)

watch(
  () => time.value,
  (v) => {
    if (!model.value) {
      model.value = new Date()
    }

    if (v) {
      const [hours, minutes] = v.split(':')
      model.value.setHours(parseInt(hours))
      model.value.setMinutes(parseInt(minutes))
      model.value.setSeconds(0)
      model.value.setMilliseconds(0)
    }
  },
  { immediate: true }
)

const init = () => {
  if (!model.value) return

  date.value = format(model.value, 'yyyy-MM-dd')
  time.value = format(model.value, 'HH:mm')
}

init()
</script>
<template>
  <div>
    <input
      type="date"
      v-model="date"
      :disabled="props.disabled"
      :min="props.min ? format(props.min, 'yyyy-MM-dd') : undefined"
      class="w-8/12 px-4 py-2 transition duration-150 ease-in-out rounded-md shadow-sm bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-600 text-brownish-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-offset-0 focus:ring-redish-500"
    />
    <input
      type="time"
      v-model="time"
      :disabled="props.disabled"
      class="w-4/12 px-4 py-2 transition duration-150 ease-in-out rounded-md shadow-sm bg-white dark:bg-brownish-800 border border-paper-300 dark:border-brownish-600 text-brownish-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-offset-0 focus:ring-redish-500"
    />
  </div>
</template>
