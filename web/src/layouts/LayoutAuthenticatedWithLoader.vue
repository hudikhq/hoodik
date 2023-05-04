<script setup lang="ts">
import LayoutAuthenticated from './LayoutAuthenticated.vue'
</script>

<template>
  <Suspense>
    <LayoutAuthenticated v-slot="{ authenticated, keypair }">
      <slot :authenticated="authenticated" :keypair="keypair" />
    </LayoutAuthenticated>

    <template #fallback>
      <div
        class="flex min-h-screen items-center justify-center bg-gradient-to-tr from-brownish-700 via-brownish-900 to-brownish-800"
      >
        <div class="rounded-2xl flex-col flex bg-brownish-700 element">
          <div class="flex-1 p-4">
            <img src="public/android-chrome-192x192.png" alt="logo" />
            <div class="working">
              Working<span class="dot">.</span><span class="dot">.</span><span class="dot">.</span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </Suspense>
</template>
<style lang="css">
.element {
  border-radius: 50%;
  box-shadow: 0 0 0 0 rgba(249, 38, 114, 0.7);
  border: 10px solid #a63446;
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
  animation: dot 1s infinite;
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
