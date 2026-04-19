/**
 * Standalone editor entry point for Flutter webview.
 *
 * Creates a Milkdown editor without Vue, using Editor.make() directly.
 * Communicates with the Flutter host via the EditorBridgeHost.
 */
import { Editor, rootCtx } from '@milkdown/core'
import { configureEditor, getBasePlugins } from '../setup'
import { EditorBridgeHost } from './bridge'
import '../styles/editor.css'

const bridge = new EditorBridgeHost()

// Expose message receiver for Flutter's runJavaScript
window.hoodik = {
  receiveMessage: (json: string) => {
    try {
      const msg = JSON.parse(json)
      bridge.handleMessage(msg)
    } catch (err) {
      console.error('[EditorBridge] Failed to parse message:', err)
    }
  },
}

async function init() {
  const container = document.getElementById('editor')
  if (!container) {
    console.error('[Editor] #editor container not found')
    return
  }

  const options = {
    content: '',
    editable: true,
    callbacks: {
      onContentChanged: (markdown: string) => bridge.notifyContentChanged(markdown),
      onSave: () => bridge.notifySaveRequested(),
    },
  }

  try {
    let builder = Editor.make()
      .config((ctx) => {
        ctx.set(rootCtx, container)
        configureEditor(ctx, options)
      })

    for (const plugin of getBasePlugins(options)) {
      builder = builder.use(plugin)
    }

    const editor = await builder.create()

    bridge.attach(editor)
    bridge.postReady()
  } catch (err) {
    console.error('[Editor] Failed to initialize:', err)
  }
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init)
} else {
  init()
}
