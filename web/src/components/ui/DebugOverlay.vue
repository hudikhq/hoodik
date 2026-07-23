<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { formatSize } from '!'
import { wasmMemoryBytes } from '!/cryptfns/wasm'
import { store as downloadStore } from '!/storage/download'
import { store as uploadStore } from '!/storage/upload'

/**
 * Always-on-top diagnostics for the crypto/transfer machinery. Hidden
 * unless enabled; toggle with Ctrl+Shift+D (persisted in localStorage).
 *
 * The wasm figures matter because linear memory never shrinks and wasm32
 * caps at 4 GB — the download worker assembles whole files in memory, so
 * this is where a too-large transfer shows up first.
 */
const STORAGE_KEY = 'hoodik:debug'

const enabled = ref(false)
const mainWasm = ref(0)
const mainHeap = ref<{ used: number; limit: number }>()
const workerWasm = ref<Record<string, number>>({})

const download = downloadStore()
const upload = uploadStore()

let timer: ReturnType<typeof setInterval> | undefined

const workers = () =>
  [window.DOWNLOAD, window.UPLOAD].filter((worker): worker is Worker => !!worker)

function onWorkerMessage(event: MessageEvent) {
  if (event.data?.type !== 'debug-stats') return

  const { name, wasmBytes } = event.data.response
  workerWasm.value = { ...workerWasm.value, [name]: wasmBytes }
}

function sample() {
  mainWasm.value = wasmMemoryBytes()

  // Chrome-only; other engines simply don't get the heap line.
  const memory = (performance as { memory?: { usedJSHeapSize: number; jsHeapSizeLimit: number } })
    .memory
  mainHeap.value = memory ? { used: memory.usedJSHeapSize, limit: memory.jsHeapSizeLimit } : undefined

  workers().forEach((worker) => worker.postMessage({ type: 'debug-stats' }))
}

function start() {
  if (timer) return
  workers().forEach((worker) => worker.addEventListener('message', onWorkerMessage))
  sample()
  timer = setInterval(sample, 1000)
}

function stop() {
  if (!timer) return
  clearInterval(timer)
  timer = undefined
  workers().forEach((worker) => worker.removeEventListener('message', onWorkerMessage))
}

function toggle(event: KeyboardEvent) {
  if (!(event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'd')) return

  event.preventDefault()
  enabled.value = !enabled.value
  localStorage.setItem(STORAGE_KEY, enabled.value ? '1' : '0')
  enabled.value ? start() : stop()
}

const transfers = computed(() => {
  const parts = [
    `↓ ${download.running.length} active / ${download.waiting.length} queued`,
    `↑ ${upload.running.length} active / ${upload.waiting.length} queued`
  ]
  return parts.join('   ')
})

onMounted(() => {
  window.addEventListener('keydown', toggle)
  enabled.value = localStorage.getItem(STORAGE_KEY) === '1'
  if (enabled.value) start()
})

onUnmounted(() => {
  stop()
  window.removeEventListener('keydown', toggle)
})
</script>

<template>
  <div
    v-if="enabled"
    class="fixed bottom-2 left-2 z-[100] px-3 py-2 rounded-lg font-mono text-[11px] leading-relaxed
      pointer-events-none select-none
      bg-white/85 dark:bg-brownish-900/85 backdrop-blur-sm
      border border-brownish-200/40 dark:border-brownish-600/60
      text-brownish-400 dark:text-brownish-100"
  >
    <div>wasm main: {{ formatSize(mainWasm) }}</div>
    <div v-for="(bytes, name) in workerWasm" :key="name">
      wasm {{ String(name).toLowerCase().includes('download') ? '↓' : '↑' }}:
      {{ formatSize(bytes) }}
    </div>
    <div v-if="mainHeap">heap: {{ formatSize(mainHeap.used) }} / {{ formatSize(mainHeap.limit) }}</div>
    <div>{{ transfers }}</div>
  </div>
</template>
