<script setup lang="ts">
import { watch } from 'vue'
import { Milkdown, useEditor, useInstance } from '@milkdown/vue'
import { Editor, defaultValueCtx, editorViewOptionsCtx, rootCtx } from '@milkdown/core'
import { commonmark } from '@milkdown/preset-commonmark'
import { gfm } from '@milkdown/preset-gfm'
import { listener, listenerCtx } from '@milkdown/plugin-listener'
import { prism } from '@milkdown/plugin-prism'
import { history } from '@milkdown/plugin-history'
import { nord } from '@milkdown/theme-nord'
import { replaceAll, callCommand } from '@milkdown/utils'
import { createKeyboardShortcutsPlugin } from './plugins/keyboard-shortcuts'
import { htmlRenderView } from './plugins/html-render'
import { createHeadingAnchorPlugin } from './plugins/heading-anchor'

const props = defineProps<{
  content: string
  editable: boolean
}>()

const emit = defineEmits<{
  (event: 'update:content', value: string): void
  (event: 'save'): void
}>()

const shortcutsPlugin = createKeyboardShortcutsPlugin({
  onSave: () => emit('save')
})

useEditor((container) => {
  const editor = Editor
    .make()
    .config((ctx) => {
      ctx.set(rootCtx, container)
      ctx.set(defaultValueCtx, props.content)
      nord(ctx)

      ctx.update(editorViewOptionsCtx, (prev) => ({
        ...prev,
        editable: () => props.editable,
        attributes: {
          'data-gramm': 'false',
          'data-gramm_editor': 'false',
          'data-enable-grammarly': 'false'
        }
      }))

      ctx.get(listenerCtx).markdownUpdated((_ctx, markdown, prevMarkdown) => {
        if (markdown !== prevMarkdown) {
          lastContent = markdown
          emit('update:content', markdown)
        }
      })
    })
    .use(commonmark)
    .use(gfm)
    .use(listener)
    .use(prism)
    .use(history)
    .use(htmlRenderView)
    .use(createHeadingAnchorPlugin())

  if (props.editable) {
    editor.use(shortcutsPlugin)
  }

  return editor
})

const [loadingRef, getEditor] = useInstance()

function runCommand(commandKey: string, payload?: unknown) {
  if (loadingRef.value) return
  const editor = getEditor()
  if (!editor) return

  editor.action(callCommand(commandKey, payload))
}

let lastContent = props.content
watch(
  () => props.content,
  (newContent) => {
    if (newContent !== lastContent && !loadingRef.value) {
      const editor = getEditor()
      if (editor) {
        editor.action(replaceAll(newContent, true))
      }
      lastContent = newContent
    }
  }
)

defineExpose({ runCommand })
</script>

<template>
  <Milkdown />
</template>
