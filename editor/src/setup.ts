import type { Ctx } from '@milkdown/ctx'
import type { MilkdownPlugin } from '@milkdown/ctx'
import { defaultValueCtx, editorViewOptionsCtx } from '@milkdown/core'
import { commonmark } from '@milkdown/preset-commonmark'
import { gfm } from '@milkdown/preset-gfm'
import { listener, listenerCtx } from '@milkdown/plugin-listener'
import { prism } from '@milkdown/plugin-prism'
import { history } from '@milkdown/plugin-history'
import { nord } from '@milkdown/theme-nord'

import { createKeyboardShortcutsPlugin } from './plugins/keyboard-shortcuts'
import { createHeadingAnchorPlugin } from './plugins/heading-anchor'
import { htmlRenderView } from './plugins/html-render'
import { configurePrismLanguages } from './plugins/prism-languages'
import type { EditorOptions } from './types'

/**
 * Configures a Milkdown editor context with Hoodik's standard settings.
 *
 * Call this inside `Editor.make().config(ctx => configureEditor(ctx, options))`.
 * Works with both @milkdown/vue's useEditor and standalone Editor.make().
 */
export function configureEditor(ctx: Ctx, options: EditorOptions): void {
  ctx.set(defaultValueCtx, options.content)
  nord(ctx)

  ctx.update(editorViewOptionsCtx, (prev) => ({
    ...prev,
    editable: () => options.editable,
    attributes: {
      'data-gramm': 'false',
      'data-gramm_editor': 'false',
      'data-enable-grammarly': 'false',
    },
  }))

  configurePrismLanguages(ctx)

  ctx.get(listenerCtx).markdownUpdated((_ctx, markdown, prevMarkdown) => {
    if (markdown !== prevMarkdown) {
      options.callbacks.onContentChanged(markdown)
    }
  })
}

/**
 * Returns the standard set of Milkdown plugins for Hoodik's editor.
 *
 * Includes: CommonMark, GFM, listener, prism, history, HTML render,
 * heading anchors, and keyboard shortcuts (Mod-s for save in edit mode).
 *
 * Pass `options.extraPlugins` to append additional plugins (e.g. wiki-link,
 * image-upload) without modifying this function.
 */
export function getBasePlugins(options: EditorOptions): (MilkdownPlugin | MilkdownPlugin[])[] {
  const plugins: (MilkdownPlugin | MilkdownPlugin[])[] = [
    commonmark,
    gfm,
    listener,
    prism,
    history,
    htmlRenderView,
    createHeadingAnchorPlugin(),
  ]

  if (options.editable) {
    plugins.push(
      createKeyboardShortcutsPlugin({ onSave: options.callbacks.onSave })
    )
  }

  if (options.extraPlugins) {
    plugins.push(...options.extraPlugins)
  }

  return plugins
}
