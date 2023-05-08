<script setup lang="ts">
import { useForm, type SubmissionContext } from 'vee-validate'
import PuppyLoader from '../ui/PuppyLoader.vue'
import { computed } from 'vue'

const props = defineProps<{
  config: Parameters<typeof useForm>
  class?: string
  working?: boolean
  leaveOnSubmit?: boolean
}>()

const form = useForm({
  validateOnMount: false,
  ...props.config
})

const submit = form.handleSubmit(async (values, ctx: SubmissionContext<typeof values>) => {
  // @ts-ignore
  if (typeof props?.config?.onSubmit === 'function') {
    // @ts-ignore
    await props.config.onSubmit(values, ctx)
  }
})

let waiter: ReturnType<typeof setTimeout> | null = null
const debouncedFn = () => {
  if (waiter) {
    console.log('have waiter')
    return
  }

  waiter = setTimeout(() => {
    if (!form?.isSubmitting?.value) {
      console.log('posting')
      submit()
    }

    if (waiter) {
      clearTimeout(waiter)
      waiter = null
    }
  }, 1000)
}

const isWorking = computed(() => !!form?.isSubmitting?.value || !!props.working)
// const isWorking = computed(() => true)

defineExpose({
  form,
  submit,
  debouncedFn,
  isWorking
})
</script>

<template>
  <PuppyLoader v-model="isWorking" />

  <form
    :class="{
      [props.class || '']: true,
      submitting: !!isWorking
    }"
    @submit="submit"
  >
    <slot :form="{ ...form, isWorking }" :submit="submit" :debounced="debouncedFn" />
  </form>
</template>

<style scoped lang="css">
.submitting {
  opacity: 0.1;
}
</style>
