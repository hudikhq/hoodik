import type { Editor } from '@milkdown/core'
import { editorViewOptionsCtx } from '@milkdown/core'
import { getMarkdown, replaceAll, callCommand } from '@milkdown/utils'
import type { HostToEditorMessage, EditorToHostMessage } from './protocol'

declare global {
  interface Window {
    /** Injected by Flutter's JavaScriptChannel */
    HoodikBridge?: { postMessage: (json: string) => void }
    /** Exposed for Flutter's runJavaScript to call */
    hoodik: {
      receiveMessage: (json: string) => void
    }
  }
}

/**
 * Webview-side bridge that mediates between the Milkdown editor
 * and the Flutter host via JSON messages.
 */
export class EditorBridgeHost {
  private editor: Editor | null = null
  private initialContent = ''
  private lastSentContent = ''

  attach(editor: Editor): void {
    this.editor = editor
  }

  setInitialContent(content: string): void {
    this.initialContent = content
    this.lastSentContent = content
  }

  handleMessage(msg: HostToEditorMessage): void {
    if (!this.editor) {
      this.postToHost({ type: 'error', message: 'Editor not initialized' })
      return
    }

    switch (msg.type) {
      case 'setContent':
        this.initialContent = msg.markdown
        this.lastSentContent = msg.markdown
        this.editor.action(replaceAll(msg.markdown, true))
        break

      case 'setEditable':
        this.editor.action((ctx) => {
          ctx.update(editorViewOptionsCtx, (prev) => ({
            ...prev,
            editable: () => msg.editable,
          }))
        })
        // Force view to re-evaluate editability
        this.editor.action((ctx) => {
          const view = ctx.get(editorViewOptionsCtx)
          void view // trigger reconfiguration
        })
        break

      case 'getMarkdown': {
        const markdown = this.editor.action(getMarkdown())
        this.postToHost({
          type: 'markdownResult',
          requestId: msg.requestId,
          markdown,
        })
        break
      }

      case 'runCommand':
        this.editor.action(callCommand(msg.command, msg.payload))
        break

      case 'setZoom':
        this.applyZoom(msg.scale)
        break

      default:
        // Future message types (assetResolved, setNoteList, etc.)
        // will be handled here as they're implemented
        break
    }
  }

  notifyContentChanged(markdown: string): void {
    this.lastSentContent = markdown
    const isDirty = markdown !== this.initialContent
    this.postToHost({ type: 'contentChanged', markdown, isDirty })
  }

  notifySaveRequested(): void {
    this.postToHost({ type: 'saveRequested' })
  }

  postReady(): void {
    this.postToHost({ type: 'ready' })
  }

  /** Mark content as saved (resets dirty baseline) */
  markSaved(): void {
    this.initialContent = this.lastSentContent
  }

  /**
   * Scale the editor font size. Clamped between 0.5 and 3.0 so hosts
   * can't accidentally push the content off-screen. The CSS reads this
   * via `--editor-zoom` on the wrapper, which the stylesheet multiplies
   * into the root font-size.
   */
  private applyZoom(scale: number): void {
    const clamped = Math.min(3, Math.max(0.5, scale))
    const wrapper = document.querySelector<HTMLElement>('.milkdown-wrapper')
    if (wrapper) {
      wrapper.style.setProperty('--editor-zoom', clamped.toString())
    }
  }

  private postToHost(msg: EditorToHostMessage): void {
    const json = JSON.stringify(msg)
    // Flutter webview — JavaScriptChannel
    if (window.HoodikBridge?.postMessage) {
      window.HoodikBridge.postMessage(json)
      return
    }
    // Fallback for debugging in a browser
    console.debug('[EditorBridge →]', msg)
  }
}
