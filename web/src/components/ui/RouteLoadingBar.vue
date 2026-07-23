<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'
import { useRouter } from 'vue-router'

/**
 * Views are lazy route chunks, so on a slow connection a click can sit for
 * many seconds with nothing on screen while the code downloads. This bar
 * appears at the top whenever a navigation takes longer than a beat, so
 * every route change shows life immediately. Indeterminate on purpose —
 * chunk sizes aren't knowable from here, and a moving bar is the point.
 */
const router = useRouter()
const visible = ref(false)

let delay: ReturnType<typeof setTimeout> | undefined

const start = () => {
  clearTimeout(delay)
  delay = setTimeout(() => {
    visible.value = true
  }, 250)
}

const stop = () => {
  clearTimeout(delay)
  visible.value = false
}

const removeBefore = router.beforeEach(() => start())
const removeAfter = router.afterEach(() => stop())
const removeError = router.onError(() => stop())

onBeforeUnmount(() => {
  clearTimeout(delay)
  removeBefore()
  removeAfter()
  removeError()
})
</script>

<template>
  <div
    v-if="visible"
    class="fixed top-0 left-0 w-full h-0.5 z-[100] overflow-hidden pointer-events-none"
    role="progressbar"
  >
    <div class="h-full w-1/3 bg-greeny-400 route-loading-slide" />
  </div>
</template>

<style scoped>
.route-loading-slide {
  animation: route-loading-slide 1.1s ease-in-out infinite;
}

@keyframes route-loading-slide {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(400%);
  }
}
</style>
