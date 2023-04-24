<script setup lang="ts">
import { computed } from 'vue'
import { mdiClose } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtons from '@/components/ui/BaseButtons.vue'
import CardBox from '@/components/ui/CardBox.vue'
import OverlayLayer from '@/components/ui/OverlayLayer.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import type { ColorType } from '@/colors'
import type { FormType } from '../form'

const props = defineProps<{
  title: string
  button?: ColorType
  buttonLabel?: string
  hasCancel?: boolean
  hideSubmit?: boolean
  modelValue: string | number | boolean
  form?: FormType
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const value = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const confirm = async () => {
  if (!props.form) {
    value.value = false
    emit('confirm')
  }
}

const cancel = () => {
  if (props.form) {
    props.form.handleReset()
  }

  value.value = false
  emit('cancel')
}

window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && value.value) {
    cancel()
  }
})
</script>

<template>
  <OverlayLayer v-show="value" @overlay-click="cancel">
    <CardBox
      v-show="value"
      class="shadow-lg max-h-modal w-11/12 md:w-3/5 lg:w-2/5 xl:w-4/12 z-50"
      is-modal
    >
      <CardBoxComponentTitle :title="title">
        <BaseButton
          v-if="hasCancel"
          :icon="mdiClose"
          color="whiteDark"
          small
          rounded-full
          @click.prevent="cancel"
        />
      </CardBoxComponentTitle>

      <div class="space-y-3">
        <slot />
      </div>

      <template #footer>
        <BaseButtons>
          <slot name="buttons">
            <BaseButton
              v-if="!hideSubmit"
              :label="buttonLabel"
              :color="button || 'info'"
              @click="confirm"
              tabindex="1"
              type="submit"
              @keyup.enter="confirm()"
            />
            <BaseButton
              v-if="hasCancel"
              label="Cancel"
              :color="button || 'info'"
              outline
              @click="cancel"
            />
          </slot>
        </BaseButtons>
      </template>
    </CardBox>
  </OverlayLayer>
</template>
