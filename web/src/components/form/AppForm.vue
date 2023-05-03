<script setup lang="ts">
import { useForm, type SubmissionContext } from 'vee-validate'
import { mdiLoading } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { computed, ref } from 'vue'

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

defineExpose({
  form,
  submit,
  debouncedFn,
  isWorking
})
</script>

<template>
  <div>
    <BaseIcon v-if="!!isWorking" :path="mdiLoading" class="spinner" size="100" />
    <form
      :class="{
        [props.class || '']: true,
        submitting: !!isWorking
      }"
      @submit="submit"
    >
      <slot :form="{ ...form, isWorking }" :submit="submit" :debounced="debouncedFn" />
    </form>
  </div>
</template>

<style scoped lang="css">
.submitting {
  opacity: 0.1;
}
.spinner {
  margin: auto;
  width: 20px;
  height: 20px;
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  right: 0;
  opacity: 1;
  animation: rotate 1s linear infinite;
}
@keyframes rotate {
  to {
    transform: rotate(359deg); /* some browsers don't display spin when it is 360 deg */
  }
}
</style>
