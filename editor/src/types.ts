import type { MilkdownPlugin } from '@milkdown/ctx'

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

export interface EditorCallbacks {
  onContentChanged: (markdown: string) => void
  onSave: () => void
}

export interface EditorOptions {
  content: string
  editable: boolean
  callbacks: EditorCallbacks
  /** Additional plugins to load (e.g. image-upload, wiki-link) */
  extraPlugins?: MilkdownPlugin[]
}
