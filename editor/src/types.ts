import type { MilkdownPlugin } from '@milkdown/ctx'

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

export interface EditorCallbacks {
  onContentChanged: (markdown: string) => void
  onSave: () => void
  /** Cmd/Ctrl+F inside the editor. Hosts show their own find bar. */
  onFindRequested?: () => void
}

export interface EditorOptions {
  content: string
  editable: boolean
  callbacks: EditorCallbacks
  /** Additional plugins to load (e.g. image-upload, wiki-link) */
  extraPlugins?: MilkdownPlugin[]
}
