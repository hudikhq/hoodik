<script setup lang="ts">
import type { Stats } from 'types/admin/files'
import { computed } from 'vue'
import { formatSize } from '!/index'

const props = defineProps<{
  data: Stats
  max?: number
}>()

const label = computed(() => {
  const mime = props.data.mime
  if (!mime) return 'Unknown'
  const parts = mime.split('/')
  if (parts.length !== 2) return mime
  const [type, subtype] = parts

  const subtypeMap: Record<string, string> = {
    'jpeg': 'JPEG', 'jpg': 'JPEG', 'png': 'PNG', 'gif': 'GIF', 'webp': 'WebP',
    'svg+xml': 'SVG', 'mp4': 'MP4', 'webm': 'WebM', 'ogg': 'OGG', 'mp3': 'MP3',
    'mpeg': 'MPEG', 'wav': 'WAV', 'flac': 'FLAC', 'aac': 'AAC',
    'pdf': 'PDF', 'zip': 'ZIP', 'x-zip-compressed': 'ZIP', 'gzip': 'GZip',
    'json': 'JSON', 'xml': 'XML', 'html': 'HTML', 'plain': 'Text',
    'msword': 'Word', 'octet-stream': 'Binary', 'x-tar': 'TAR',
    'vnd.openxmlformats-officedocument.wordprocessingml.document': 'DOCX',
    'vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'XLSX',
    'vnd.openxmlformats-officedocument.presentationml.presentation': 'PPTX',
    'x-matroska': 'MKV', 'quicktime': 'MOV', 'x-msvideo': 'AVI',
    'tiff': 'TIFF', 'bmp': 'BMP', 'ico': 'ICO', 'heic': 'HEIC', 'heif': 'HEIF',
  }

  const prettySubtype = subtypeMap[subtype] ?? subtype.split(/[+.-]/).pop()?.toUpperCase() ?? subtype.toUpperCase()
  const typeLabels: Record<string, string> = {
    'image': 'Images', 'video': 'Videos', 'audio': 'Audio',
    'text': 'Text', 'application': '', 'font': 'Fonts'
  }
  const typeLabel = typeLabels[type] ?? type
  return typeLabel ? `${prettySubtype} ${typeLabel}` : prettySubtype
})

const count = computed(() => props.data.count)
const size = computed(() => formatSize(props.data.size))
const percentage = computed(() => {
  if (!props.data.size || !props.max) return 0
  return Math.min(100, Math.round((props.data.size / props.max) * 100))
})
const percentageStr = computed(() => `${percentage.value}%`)
const maxSize = computed(() => props.max ? formatSize(props.max) : 'unlimited')
</script>
<template>
  <div
    class="py-2 border-b border-brownish-100 dark:border-brownish-700/50 last:border-0"
    :title="`${percentageStr} of ${maxSize}`"
  >
    <div class="flex items-center gap-2">
      <span class="flex-1 min-w-0 truncate text-sm">{{ label }}</span>
      <span class="text-xs text-brownish-400 shrink-0">{{ count }}×</span>
      <span class="text-xs font-medium w-14 text-right shrink-0">{{ size }}</span>
    </div>
    <div v-if="max" class="mt-1.5 h-1 bg-brownish-100 dark:bg-brownish-700 rounded-full overflow-hidden">
      <div
        class="h-1 bg-greeny-500/60 rounded-full transition-[width] duration-500"
        :style="{ width: percentageStr }"
      />
    </div>
  </div>
</template>
