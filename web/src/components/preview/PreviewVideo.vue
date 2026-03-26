<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import type { Preview } from '!/preview'
import SpinnerIcon from '../ui/SpinnerIcon.vue'
import * as logger from '!/logger'

const props = defineProps<{
  modelValue: Preview
}>()

const blobUrl = ref<string>()
const progress = ref(0)
const error = ref<string>()

let mediaSource: MediaSource | null = null

function cleanup() {
  if (blobUrl.value) {
    URL.revokeObjectURL(blobUrl.value)
    blobUrl.value = undefined
  }
  if (mediaSource?.readyState === 'open') {
    try { mediaSource.endOfStream() } catch { /* endOfStream throws if the stream is already closed */ }
  }
  mediaSource = null
}

/**
 * Returns the bare container MIME string to pass to addSourceBuffer(), or null
 * if MSE cannot handle this format at all. We intentionally avoid codec
 * strings (e.g. avc1.42E01E) because isTypeSupported() can return true for
 * H.264 Baseline while the actual video is HEVC or H.264 High Profile —
 * causing appendBuffer() to fail. Bare types let the browser auto-detect the
 * codec from the bitstream.
 */
function getCodecMime(mime: string): string | null {
  if (!('MediaSource' in window)) {
    logger.info('[video-preview] MediaSource API not available in this browser')
    return null
  }
  const mseType: Record<string, string> = {
    'video/mp4':       'video/mp4',
    'video/quicktime': 'video/mp4',  // Chrome/Firefox map QuickTime → mp4 in MSE
    'video/webm':      'video/webm',
  }
  const t = mseType[mime]
  if (!t) {
    logger.info(`[video-preview] MIME type "${mime}" has no MSE mapping — will use full download`)
    return null
  }
  const supported = MediaSource.isTypeSupported(t)
  logger.info(`[video-preview] MediaSource.isTypeSupported("${t}") = ${supported}`)
  return supported ? t : null
}

/**
 * Scan the first chunk of an MP4 file to see if `moov` appears before any
 * large `mdat`. Fast-start MP4s (phones, modern encoders) put moov first so
 * MSE can start playing after just the first chunk. Non-fast-start files have
 * moov at the end, meaning MSE offers no streaming benefit.
 */
function hasMoovBeforeMdat(data: Uint8Array): boolean {
  let offset = 0
  while (offset + 8 <= data.length) {
    const size =
      ((data[offset] << 24) | (data[offset + 1] << 16) | (data[offset + 2] << 8) | data[offset + 3]) >>> 0
    const type = String.fromCharCode(
      data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
    )
    if (type === 'moov') return true
    if (type === 'mdat') return false  // mdat came first → non-fast-start
    if (size === 0 || size < 8) break   // end-of-file box or corrupt
    offset += size
  }
  return false
}

async function load() {
  cleanup()
  progress.value = 0
  error.value = undefined

  const preview = props.modelValue
  const totalChunks = preview.chunks

  logger.info(`[video-preview] opening "${preview.name}" — mime=${preview.mime} chunks=${totalChunks}`)

  if (!totalChunks) {
    // No per-chunk API (e.g. LinkPreview) — download full buffer at once
    logger.info('[video-preview] no chunk API available — falling back to preview.load()')
    try {
      const data = await preview.load()
      blobUrl.value = URL.createObjectURL(new Blob([data], { type: preview.mime }))
      progress.value = 100
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load video'
      logger.error('[video-preview] preview.load() failed', e)
    }
    return
  }

  const codecMime = getCodecMime(preview.mime)
  if (!codecMime) {
    logger.info('[video-preview] no MSE support for this format — using full download')
    await downloadFull(preview, totalChunks)
    return
  }

  // For MP4/QuickTime: pre-fetch chunk 0 to detect fast-start (moov before mdat).
  // We always attempt MSE regardless — chunk0 is passed through so it isn't re-fetched.
  // If MSE fails mid-stream (e.g. non-fast-start moov at end, unsupported codec),
  // streamMSE's catch block falls back to downloadFull() with chunk0 already in hand.
  if (preview.mime === 'video/mp4' || preview.mime === 'video/quicktime') {
    logger.info('[video-preview] MP4/QuickTime detected — fetching chunk 0 to check for fast-start')
    const chunk0 = await preview.loadChunk(0)
    progress.value = Math.round((1 / totalChunks) * 100)
    const fastStart = hasMoovBeforeMdat(chunk0)
    logger.info(`[video-preview] fast-start (moov before mdat) = ${fastStart} — attempting MSE stream regardless`)
    await streamMSE(preview, totalChunks, codecMime, chunk0)
  } else {
    logger.info(`[video-preview] streaming "${preview.mime}" via MSE with "${codecMime}"`)
    await streamMSE(preview, totalChunks, codecMime)
  }
}

/**
 * Stream via MediaSource Extensions: the <video> element becomes visible immediately
 * and starts playing as soon as the browser has buffered enough data (usually after
 * the first chunk). Falls back to downloadFull() if appendBuffer throws
 * (e.g. non-fast-start MP4 with moov at end, or unsupported codec variant).
 */
function appendChunk(sb: SourceBuffer, chunk: Uint8Array): Promise<void> {
  return new Promise((resolve, reject) => {
    sb.addEventListener('updateend', () => resolve(), { once: true })
    sb.addEventListener('error', (e) => reject(e), { once: true })
    sb.appendBuffer(chunk)
  })
}

async function streamMSE(preview: Preview, totalChunks: number, codecMime: string, chunk0?: Uint8Array) {
  logger.info(`[video-preview] streamMSE: starting MSE stream — codecMime="${codecMime}" totalChunks=${totalChunks}`)
  mediaSource = new MediaSource()
  blobUrl.value = URL.createObjectURL(mediaSource)
  const ms = mediaSource

  try {
    const sb = await new Promise<SourceBuffer>((resolve, reject) => {
      ms.addEventListener('sourceopen', () => {
        logger.info('[video-preview] streamMSE: sourceopen fired — adding SourceBuffer')
        try {
          const buf = ms.addSourceBuffer(codecMime)
          logger.info('[video-preview] streamMSE: addSourceBuffer succeeded')
          resolve(buf)
        } catch (e) {
          logger.error('[video-preview] streamMSE: addSourceBuffer failed', e)
          reject(e)
        }
      }, { once: true })
      ms.addEventListener('error', (e) => {
        logger.error('[video-preview] streamMSE: MediaSource error event', e)
        reject(e)
      }, { once: true })
    })

    for (let i = 0; i < totalChunks; i++) {
      const chunk = i === 0 && chunk0 ? chunk0 : await preview.loadChunk(i)
      logger.info(`[video-preview] streamMSE: appending chunk ${i + 1}/${totalChunks} (${chunk.length} bytes)`)
      await appendChunk(sb, chunk)
      progress.value = Math.round(((i + 1) / totalChunks) * 100)
    }

    if (ms.readyState === 'open') {
      logger.info('[video-preview] streamMSE: all chunks appended — calling endOfStream()')
      ms.endOfStream()
    }
  } catch (e) {
    // Container incompatible with MSE — fall back to full download.
    // Pass chunk0 so it isn't re-fetched.
    logger.error('[video-preview] streamMSE: MSE streaming failed — falling back to full download', e)
    cleanup()
    await downloadFull(preview, totalChunks, chunk0)
  }
}

/**
 * Fallback: download all chunks, concatenate, then create a blob URL.
 * chunk0 may already be in hand (passed from pre-fetch or failed MSE attempt).
 */
async function downloadFull(preview: Preview, totalChunks: number, chunk0?: Uint8Array) {
  logger.info(`[video-preview] downloadFull: starting full download — totalChunks=${totalChunks} chunk0=${chunk0 ? 'provided' : 'none'}`)
  try {
    const parts: Uint8Array[] = chunk0 ? [chunk0] : []
    let totalBytes = chunk0 ? chunk0.length : 0

    for (let i = chunk0 ? 1 : 0; i < totalChunks; i++) {
      const chunk = await preview.loadChunk(i)
      parts.push(chunk)
      totalBytes += chunk.length
      progress.value = Math.round(((i + 1) / totalChunks) * 100)
      logger.info(`[video-preview] downloadFull: chunk ${i + 1}/${totalChunks} downloaded (${chunk.length} bytes, total so far ${totalBytes})`)
    }
    const combined = new Uint8Array(totalBytes)
    let offset = 0
    for (const p of parts) {
      combined.set(p, offset)
      offset += p.length
    }
    logger.info(`[video-preview] downloadFull: all chunks done — creating blob URL (${totalBytes} bytes total)`)
    blobUrl.value = URL.createObjectURL(new Blob([combined], { type: preview.mime }))
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load video'
    logger.error('[video-preview] downloadFull: failed', e)
  }
}

watch(() => props.modelValue, load, { immediate: true })
onUnmounted(cleanup)
</script>

<template>
  <!-- Full-download loading state: shown until blob URL is ready -->
  <div v-if="!blobUrl && !error" class="flex flex-col items-center justify-center w-full h-full gap-4">
    <SpinnerIcon />
    <div class="text-sm text-gray-400">Loading… {{ progress }}%</div>
    <div class="w-64 h-2 bg-gray-700 rounded-full overflow-hidden">
      <div class="h-full bg-blue-500 transition-all" :style="{ width: `${progress}%` }" />
    </div>
  </div>

  <div v-else-if="error" class="flex flex-col items-center justify-center w-full h-full gap-2 text-red-400">
    <span>Could not load video: {{ error }}</span>
  </div>

  <!-- Video visible: in MSE mode this appears immediately; in fallback mode after full download -->
  <div v-else class="flex flex-col items-center w-full h-full">
    <video :src="blobUrl" controls autoplay class="max-w-full max-h-[calc(100%-2.5rem)]" />
    <!-- Progress bar shown while MSE is still streaming remaining chunks -->
    <div v-if="progress < 100" class="w-64 h-2 bg-gray-700 rounded-full overflow-hidden mt-2">
      <div class="h-full bg-blue-500 transition-all" :style="{ width: `${progress}%` }" />
    </div>
  </div>
</template>
