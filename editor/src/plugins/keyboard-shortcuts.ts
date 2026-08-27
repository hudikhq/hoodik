import type { MilkdownPlugin, Ctx } from '@milkdown/ctx'
import { InitReady, prosePluginsCtx } from '@milkdown/core'
import { keymap } from '@milkdown/prose/keymap'

export interface KeyboardShortcutCallbacks {
  onSave?: () => void
  onFindRequested?: () => void
  onCloseTabRequested?: () => void
}

export function createKeyboardShortcutsPlugin(
  callbacks: KeyboardShortcutCallbacks
): MilkdownPlugin {
  const plugin: MilkdownPlugin = (ctx: Ctx) => {
    return async () => {
      await ctx.wait(InitReady)

      const bindings: Record<string, () => boolean> = {}

      if (callbacks.onSave) {
        bindings['Mod-s'] = () => {
          callbacks.onSave!()
          return true
        }
      }

      if (callbacks.onFindRequested) {
        // Returning true prevents the browser find bar.
        bindings['Mod-f'] = () => {
          callbacks.onFindRequested!()
          return true
        }
      }

      if (callbacks.onCloseTabRequested) {
        // Returning true keeps Cmd/Ctrl+W from the host window-close path.
        bindings['Mod-w'] = () => {
          callbacks.onCloseTabRequested!()
          return true
        }
      }

      const keymapPlugin = keymap(bindings)

      ctx.update(prosePluginsCtx, (plugins) => [...plugins, keymapPlugin])
    }
  }

  plugin.meta = {
    package: '@hoodik/keyboard-shortcuts',
    displayName: 'Keyboard Shortcuts'
  }

  return plugin
}
