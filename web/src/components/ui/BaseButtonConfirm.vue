<script setup lang="ts">
import BaseButton from './BaseButton.vue'
import type { ColorType } from '@/colors'
import { ref } from 'vue'
import type { RouteLocation } from 'vue-router'
import { mdiCancel } from '@mdi/js'

const props = defineProps<{
  confirmLabel: string
  cancelLabel?: string
  label?: string | number
  icon?: string
  iconSize?: number
  href?: string
  target?: string
  to?: RouteLocation | { name: string }
  type?: string
  color?: ColorType
  as?: string
  xs?: Boolean
  small?: Boolean
  outline?: Boolean
  active?: Boolean
  disabled?: Boolean
  roundedFull?: Boolean
  noBorder?: Boolean
  class?: String
  dropdownEl?: boolean
}>()

const emits = defineEmits<{
  (event: 'confirm'): void
  (event: 'cancel'): void
}>()

const clicked = ref(false)

const cancel = () => {
  clicked.value = false
  emits('cancel')
}
</script>

<template>
  <Transition name="fade">
    <div class="inline-block" :class="props.class">
      <BaseButton
        v-if="!clicked"
        :label="props.label"
        :icon="props.icon"
        :iconSize="props.iconSize"
        :href="props.href"
        :target="props.target"
        :to="props.to"
        :type="props.type"
        :color="props.color"
        :as="props.as"
        :xs="props.xs"
        :small="props.small"
        :outline="props.outline"
        :active="props.active"
        :disabled="props.disabled"
        :roundedFull="props.roundedFull"
        :noBorder="props.noBorder"
        :class="props.class"
        :dropdownEl="props.dropdownEl"
        @click="clicked = true"
      />

      <div class="inline-block rounded overflow-clip" v-else>
        <BaseButton
          :label="props.confirmLabel"
          :icon="props.icon"
          :iconSize="props.iconSize"
          :href="props.href"
          :target="props.target"
          :to="props.to"
          :type="props.type"
          :color="props.color"
          :as="props.as"
          :xs="props.xs"
          :small="props.small"
          :outline="props.outline"
          :active="props.active"
          :disabled="props.disabled"
          :roundedFull="false"
          :notRounded="true"
          :noBorder="props.noBorder"
          @click="emits('confirm')"
        />
        <BaseButton
          :label="props.cancelLabel ?? ''"
          :icon="mdiCancel"
          :iconSize="props.iconSize"
          :href="props.href"
          :target="props.target"
          :to="props.to"
          :type="props.type"
          color="light"
          :as="props.as"
          :xs="props.xs"
          :small="props.small"
          :outline="props.outline"
          :active="props.active"
          :disabled="props.disabled"
          :roundedFull="false"
          :notRounded="true"
          :noBorder="props.noBorder"
          @click="cancel"
        />
      </div>
    </div>
  </Transition>
</template>
<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.5s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
