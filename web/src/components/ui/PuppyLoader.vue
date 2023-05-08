<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  modelValue?: boolean
  opacity?: number
}>()

const opacity = computed(() => {
  return props.opacity || 1
})
</script>
<template>
  <div
    class="puppy-loader-div bg-brownish-700"
    :style="{ opacity }"
    v-if="props.modelValue || typeof props.modelValue === 'undefined'"
  >
    <img src="/android-chrome-192x192.png" alt="logo" />

    <div class="working">
      Working<span class="dot">.</span><span class="dot">.</span><span class="dot">.</span>
    </div>
  </div>
</template>
<style scoped lang="css">
.puppy-loader-div {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  padding: 20px;
  border-radius: 10px;
  border-radius: 25%;
  animation: breath 2s infinite;
}

@keyframes breath {
  0% {
    box-shadow: 0 0 0 0 rgba(249, 38, 114, 0.7);
  }
  50% {
    box-shadow: 0 0 0 50px rgba(249, 38, 114, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(249, 38, 114, 0.7);
  }
}

.working {
  font-size: 20px;
  display: inline-block;
  position: relative;
  color: #eeeeee;
  text-align: center;
  width: 100%;
}

.dot {
  display: inline-block;
  opacity: 0;
  animation: dot 0.6s infinite;
  animation-delay: 0;
  width: 10px;
  text-align: center;
}

.dot:nth-child(2) {
  animation-delay: 0.2s;
}

.dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes dot {
  0% {
    opacity: 0;
  }
  50% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }
}
</style>
