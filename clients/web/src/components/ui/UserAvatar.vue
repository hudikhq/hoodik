<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps({
  email: {
    type: String,
    required: false
  },
  api: {
    type: String,
    default: 'avataaars'
  }
})

const avatar = computed(
  () =>
    `https://avatars.dicebear.com/api/${props.api}/${(props.email || '').replace(
      /[^a-z0-9]+/i,
      '-'
    )}.svg`
)

const email = computed(() => props.email)
</script>

<template>
  <div>
    <img
      v-if="email"
      :src="avatar"
      :alt="email"
      class="rounded-full block h-auto w-full max-w-full bg-brownish-100 dark:bg-brownish-800"
    />
    <slot />
  </div>
</template>
