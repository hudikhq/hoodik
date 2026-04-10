<script setup lang="ts">
import { ref } from 'vue'
import {
  mdiDotsVertical,
  mdiEye,
  mdiEyeOff,
  mdiPencilOutline,
  mdiDeleteOutline,
  mdiFileMove,
  mdiDownload,
  mdiLink,
  mdiFilePdfBox,
  mdiInformationSlabCircleOutline,
  mdiNotePlusOutline
} from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'

defineProps<{
  showRaw: boolean
  viewToggleLabel: string
  canConvertToNote: boolean
}>()

const emit = defineEmits<{
  (event: 'details'): void
  (event: 'rename'): void
  (event: 'download'): void
  (event: 'link'): void
  (event: 'export-pdf'): void
  (event: 'toggle-raw'): void
  (event: 'convert'): void
  (event: 'move'): void
  (event: 'delete'): void
}>()

const showMenu = ref(false)

function action(eventName: string) {
  emit(eventName as any)
  showMenu.value = false
}

function closeDropdown(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.actions-dropdown')) showMenu.value = false
}

defineExpose({ closeDropdown })
</script>

<template>
  <div class="relative actions-dropdown">
    <BaseButton
      color="dark"
      :icon="mdiDotsVertical"
      xs
      title="More actions"
      name="md-actions"
      @click.stop="showMenu = !showMenu"
    />
    <div v-if="showMenu" class="md-actions-dropdown">
      <button class="md-actions-option" @click="action('details')">
        <BaseIcon :path="mdiInformationSlabCircleOutline" :size="16" />
        Details
      </button>
      <button class="md-actions-option" @click="action('rename')">
        <BaseIcon :path="mdiPencilOutline" :size="16" />
        Rename
      </button>
      <button class="md-actions-option" @click="action('download')">
        <BaseIcon :path="mdiDownload" :size="16" />
        Download
      </button>
      <button class="md-actions-option" @click="action('link')">
        <BaseIcon :path="mdiLink" :size="16" />
        Public link
      </button>
      <button class="md-actions-option" @click="action('export-pdf')">
        <BaseIcon :path="mdiFilePdfBox" :size="16" />
        Export PDF
      </button>
      <button class="md-actions-option" @click="action('toggle-raw')">
        <BaseIcon :path="showRaw ? mdiEye : mdiEyeOff" :size="16" />
        {{ viewToggleLabel }}
      </button>
      <button v-if="canConvertToNote" class="md-actions-option md-actions-convert" @click="action('convert')">
        <BaseIcon :path="mdiNotePlusOutline" :size="16" />
        Convert to note
      </button>
      <button class="md-actions-option" @click="action('move')">
        <BaseIcon :path="mdiFileMove" :size="16" />
        Move to...
      </button>
      <button class="md-actions-option md-actions-danger" @click="action('delete')">
        <BaseIcon :path="mdiDeleteOutline" :size="16" />
        Delete
      </button>
    </div>
  </div>
</template>

<style scoped>
.md-actions-dropdown {
  position: absolute;
  top: 100%;
  right: 0;
  min-width: 160px;
  margin-top: 0.25rem;
  background: #232323;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 0.5rem;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  overflow: hidden;
  z-index: 50;
}

.md-actions-option {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  text-align: left;
  padding: 0.5rem 0.75rem;
  color: #b0b0b0;
  font-size: 0.8125rem;
  transition: all 150ms;
}

.md-actions-option:hover {
  background: rgba(238, 155, 92, 0.1);
  color: #EE9B5C;
}

.md-actions-convert { color: #8BA9E0; }
.md-actions-convert:hover { background: rgba(139, 169, 224, 0.1); color: #a3bfe8; }

.md-actions-danger { color: #e05555; }
.md-actions-danger:hover { background: rgba(224, 85, 85, 0.1); color: #f06060; }
</style>
