// Plugins
export { createKeyboardShortcutsPlugin } from './plugins/keyboard-shortcuts'
export type { KeyboardShortcutCallbacks } from './plugins/keyboard-shortcuts'
export { createHeadingAnchorPlugin } from './plugins/heading-anchor'
export { htmlRenderView } from './plugins/html-render'

// Editor setup
export { configureEditor, getBasePlugins } from './setup'

// Commands
export { EditorCommands } from './commands'
export type { EditorCommand } from './commands'

// Types
export type { EditorCallbacks, EditorOptions, SaveStatus } from './types'

// Bridge protocol (for Flutter integration)
export type {
  HostToEditorMessage,
  EditorToHostMessage,
} from './standalone/protocol'
