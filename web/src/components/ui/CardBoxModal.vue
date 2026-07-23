<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { mdiClose } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtons from '@/components/ui/BaseButtons.vue'
import CardBox from '@/components/ui/CardBox.vue'
import OverlayLayer from '@/components/ui/OverlayLayer.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import type { ColorType } from '@/colors'
import type { FormType } from '../form'

const props = defineProps<{
  title?: string
  button?: ColorType
  buttonLabel?: string
  hasCancel?: boolean
  hasClose?: boolean
  hideSubmit?: boolean
  modelValue: boolean | undefined
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

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && value.value) {
    cancel()
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <OverlayLayer :visible="value" @overlay-click="cancel">
    <CardBox
      v-show="value"
      class="shadow-lg max-h-modal w-11/12 md:w-3/5 lg:w-2/5 xl:w-4/12 z-50"
      is-modal
      has-component-layout
    >
      <!-- relative: lets a modal's own content render a full-cover overlay (e.g. the share submit blur) -->
      <div class="relative flex-1 min-h-0 overflow-y-auto p-4">
        <CardBoxComponentTitle v-if="title" :title="title">
          <BaseButton
            v-if="hasCancel || hasClose"
            :icon="mdiClose"
            color="dark"
            small
            rounded-full
            @click.prevent="cancel"
          />
        </CardBoxComponentTitle>
        <slot />
      </div>

      <footer v-if="!hideSubmit || hasCancel" class="p-6">
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
      </footer>
    </CardBox>
  </OverlayLayer>
</template>
