<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'
import { mdiFileQuestionOutline } from '@mdi/js'
import type { Preview } from '!/preview'
import { decodeText, TEXT_PREVIEW_MAX_BYTES } from '!/preview/text'
import { formatSize } from '!/index'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import SpinnerIcon from '../ui/SpinnerIcon.vue'

const props = defineProps<{
  modelValue: Preview
  wrap?: boolean
}>()

// Frozen and shallow: tens of thousands of lines don't need reactivity.
const lines = shallowRef<readonly string[]>()
const truncated = ref(false)
const binary = ref(false)
const error = ref<string>()
const loadedBytes = ref(0)

const downloadPercent = computed(() => {
  const size = props.modelValue.size
  if (!size || !loadedBytes.value) return 0

  return Math.min(Math.round((loadedBytes.value / size) * 100), 99)
})

const gutterWidth = computed(() => `${String(lines.value?.length ?? 1).length + 2}ch`)

let loadToken = 0

async function load() {
  const token = ++loadToken
  const preview = props.modelValue

  lines.value = undefined
  truncated.value = false
  binary.value = false
  error.value = undefined
  loadedBytes.value = 0

  try {
    // Only the head of a large file is shown, so only its first chunk is
    // fetched rather than downloading and decrypting all of it.
    const partial = (preview.size ?? 0) > TEXT_PREVIEW_MAX_BYTES && (preview.chunks ?? 0) > 1
    const bytes = partial
      ? await preview.loadChunk(0)
      : await preview.load((bytes) => {
          if (token === loadToken) loadedBytes.value = bytes
        })

    if (token !== loadToken) return

    const decoded = decodeText(bytes, partial)
    binary.value = decoded.binary
    truncated.value = decoded.truncated
    lines.value = Object.freeze(decoded.lines)
  } catch (err) {
    if (token !== loadToken) return
    const failure = err as { description?: string; message?: string }
    error.value = failure.description || failure.message || String(err)
  }
}

watch(() => props.modelValue, load, { immediate: true })
</script>

<template>
  <div
    v-if="error"
    class="flex flex-col items-center justify-center w-full flex-1 gap-2 text-redish-100"
    role="alert"
  >
    <span>{{ $t('preview.text.loadError', { error }) }}</span>
  </div>

  <div v-else-if="binary" class="flex flex-col items-center justify-center w-full flex-1 gap-4">
    <BaseIcon :path="mdiFileQuestionOutline" :size="75" h="h-75" w="w-75" />
    <span>{{ $t('preview.text.binary') }}</span>
  </div>

  <div v-else-if="lines" class="flex flex-col w-full flex-1 min-h-0" data-testid="preview-text">
    <div
      class="flex flex-wrap items-center gap-x-4 gap-y-1 px-4 py-1.5 text-xs text-brownish-50 border-b border-brownish-700 bg-brownish-900"
    >
      <span>
        {{ $t('preview.text.lines', lines.length) }}
        <template v-if="modelValue.size"> · {{ formatSize(modelValue.size) }}</template>
      </span>
      <span v-if="truncated" class="text-primary-50" data-testid="preview-text-truncated">
        {{ $t('preview.text.truncated', lines.length) }}
      </span>
    </div>

    <div
      class="text-pane flex-1 min-h-0 overflow-auto bg-brownish-900"
      tabindex="0"
      data-owns-arrow-keys
      data-testid="preview-text-pane"
    >
      <div
        class="text-body"
        :class="wrap ? 'is-wrapped' : 'is-unwrapped'"
        :style="{ '--gutter': gutterWidth }"
      >
        <div v-for="(line, i) in lines" :key="i" class="text-line">
          <span class="text-content">{{ line }}</span>
        </div>
      </div>
    </div>
  </div>

  <div v-else class="flex flex-col justify-center items-center gap-2 w-full flex-1">
    <SpinnerIcon />
    <span v-if="downloadPercent > 0" class="text-sm text-brownish-50">
      {{ downloadPercent }}%
    </span>
  </div>
</template>

<style scoped>
.text-body {
  counter-reset: line;
  padding: 0.75rem 0 2rem;
  font-family: 'SF Mono', 'Fira Code', 'JetBrains Mono', 'Cascadia Code', ui-monospace, monospace;
  font-size: 0.8125rem;
  line-height: 1.6;
  tab-size: 4;
  color: #d4d4d4;
}

/* Every row as wide as the longest one, so the sticky gutter stays put
   all the way across a horizontal scroll. */
.text-body.is-unwrapped {
  width: max-content;
  min-width: 100%;
}

.text-line {
  display: flex;
}

.text-line:hover {
  background: rgba(255, 255, 255, 0.03);
}

/* A pseudo-element is never part of a selection, so copying text doesn't
   drag the line numbers along with it. */
.text-line::before {
  counter-increment: line;
  content: counter(line);
  flex: none;
  position: sticky;
  left: 0;
  width: var(--gutter);
  padding-right: 1ch;
  margin-right: 1.5ch;
  text-align: right;
  color: #555555;
  background: #181818;
  border-right: 1px solid #303030;
  user-select: none;
}

.text-line:hover::before {
  color: #939393;
}

.text-content {
  flex: 1;
  min-width: 0;
  padding-right: 1.5rem;
  white-space: pre;
}

.is-wrapped .text-content {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.text-pane::-webkit-scrollbar { width: 10px; height: 10px; }
.text-pane::-webkit-scrollbar-track { background: transparent; }
.text-pane::-webkit-scrollbar-corner { background: transparent; }
.text-pane::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 5px; }
.text-pane::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.18); }
</style>
