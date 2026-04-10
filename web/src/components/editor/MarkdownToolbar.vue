<script setup lang="ts">
import { ref } from 'vue'
import {
  mdiFormatBold,
  mdiFormatItalic,
  mdiFormatHeader1,
  mdiFormatStrikethrough,
  mdiFormatListBulleted,
  mdiFormatListNumbered,
  mdiFormatQuoteClose,
  mdiCodeTags,
  mdiLinkVariant,
  mdiTable,
  mdiContentSave,
  mdiChevronDown,
  mdiCheck,
  mdiUndo,
  mdiRedo,
  mdiAlertCircleOutline
} from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import type { SaveStatus } from './composables/useMarkdownSave'

defineProps<{
  editable: boolean
  isDirty: boolean
  isSaving: boolean
  saveStatus: SaveStatus
}>()

const emit = defineEmits<{
  (event: 'command', command: string, payload?: unknown): void
  (event: 'save'): void
}>()

const showHeadingDropdown = ref(false)

function insertHeading(level: number) {
  emit('command', 'WrapInHeading', level)
  showHeadingDropdown.value = false
}

function closeDropdown(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.heading-dropdown')) showHeadingDropdown.value = false
}

defineExpose({ closeDropdown })
</script>

<template>
  <template v-if="editable">
    <BaseButton color="dark" :icon="mdiUndo" xs title="Undo (Ctrl+Z)" name="md-undo" @click="$emit('command', 'Undo')" />
    <BaseButton color="dark" :icon="mdiRedo" xs title="Redo (Ctrl+Shift+Z)" name="md-redo" @click="$emit('command', 'Redo')" />

    <span class="md-toolbar-divider" />

    <BaseButton color="dark" :icon="mdiFormatBold" xs title="Bold (Ctrl+B)" name="md-bold" @click="$emit('command', 'ToggleStrong')" />
    <BaseButton color="dark" :icon="mdiFormatItalic" xs title="Italic (Ctrl+I)" name="md-italic" @click="$emit('command', 'ToggleEmphasis')" />
    <BaseButton color="dark" :icon="mdiFormatStrikethrough" xs title="Strikethrough" name="md-strikethrough" @click="$emit('command', 'ToggleStrikeThrough')" />

    <span class="md-toolbar-divider" />

    <div class="relative heading-dropdown">
      <BaseButton
        color="dark"
        :icon="mdiFormatHeader1"
        xs
        title="Headings"
        name="md-headings"
        @click.stop="showHeadingDropdown = !showHeadingDropdown"
      >
        <BaseIcon :path="mdiChevronDown" :size="12" />
      </BaseButton>
      <div v-if="showHeadingDropdown" class="md-heading-dropdown">
        <button
          v-for="level in [1, 2, 3, 4, 5, 6]"
          :key="level"
          class="md-heading-option"
          @click="insertHeading(level)"
        >
          <span :class="['font-semibold', level <= 2 ? 'text-base' : level <= 4 ? 'text-sm' : 'text-xs']">
            H{{ level }}
          </span>
        </button>
      </div>
    </div>

    <span class="md-toolbar-divider" />

    <BaseButton color="dark" :icon="mdiFormatListBulleted" xs title="Bullet list" name="md-bullet-list" @click="$emit('command', 'WrapInBulletList')" />
    <BaseButton color="dark" :icon="mdiFormatListNumbered" xs title="Ordered list" name="md-ordered-list" @click="$emit('command', 'WrapInOrderedList')" />
    <BaseButton color="dark" :icon="mdiFormatQuoteClose" xs title="Blockquote" name="md-blockquote" @click="$emit('command', 'WrapInBlockquote')" />

    <span class="md-toolbar-divider" />

    <BaseButton color="dark" :icon="mdiCodeTags" xs title="Code block" name="md-code-block" @click="$emit('command', 'CreateCodeBlock')" />
    <BaseButton color="dark" :icon="mdiLinkVariant" xs title="Link (Ctrl+K)" name="md-link" @click="$emit('command', 'ToggleLink')" />
    <BaseButton color="dark" :icon="mdiTable" xs title="Insert table" name="md-table" @click="$emit('command', 'InsertTable')" />
  </template>

  <span class="flex-1" />

  <!-- Save status + button (edit mode only) -->
  <template v-if="editable">
    <span v-if="saveStatus === 'saving'" class="md-save-status text-brownish-200">Saving...</span>
    <span v-else-if="saveStatus === 'saved'" class="md-save-status text-greeny-300 flex items-center gap-1">
      <BaseIcon :path="mdiCheck" :size="14" />
      Saved
    </span>
    <span v-else-if="saveStatus === 'error'" class="md-save-status text-redish-400 flex items-center gap-1">
      <BaseIcon :path="mdiAlertCircleOutline" :size="14" />
      Save failed
    </span>

    <BaseButton
      color="dark"
      :icon="mdiContentSave"
      xs
      :disabled="!isDirty || isSaving"
      title="Save (Ctrl+S)"
      name="md-save"
      @click="$emit('save')"
    />

    <span class="md-toolbar-divider" />
  </template>
</template>

<style scoped>
.md-toolbar-divider {
  width: 1px;
  height: 1.25rem;
  background: rgba(255, 255, 255, 0.08);
  margin: 0 0.25rem;
}

.md-save-status {
  font-size: 0.75rem;
  margin-right: 0.5rem;
  letter-spacing: 0.025em;
}

.md-heading-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  min-width: 100px;
  margin-top: 0.25rem;
  background: #232323;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 0.5rem;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  overflow: hidden;
  z-index: 50;
}

.md-heading-option {
  display: block;
  width: 100%;
  text-align: left;
  padding: 0.4rem 0.75rem;
  color: #b0b0b0;
  transition: all 150ms;
}

.md-heading-option:hover {
  background: rgba(238, 155, 92, 0.1);
  color: #EE9B5C;
}
</style>
