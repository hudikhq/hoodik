import type { MilkdownPlugin, Ctx } from '@milkdown/ctx'
import { InitReady, prosePluginsCtx } from '@milkdown/core'
import { keymap } from '@milkdown/prose/keymap'

export interface KeyboardShortcutCallbacks {
  onSave: () => void
}

export function createKeyboardShortcutsPlugin(
  callbacks: KeyboardShortcutCallbacks
): MilkdownPlugin {
  const plugin: MilkdownPlugin = (ctx: Ctx) => {
    return async () => {
      await ctx.wait(InitReady)

      const keymapPlugin = keymap({
        'Mod-s': () => {
          callbacks.onSave()
          return true
        }
      })

      ctx.update(prosePluginsCtx, (plugins) => [...plugins, keymapPlugin])
    }
  }

  plugin.meta = {
    package: '@hoodik/keyboard-shortcuts',
    displayName: 'Keyboard Shortcuts'
  }

  return plugin
}
