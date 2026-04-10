/**
 * Global event bus for file tree mutations.
 *
 * Components that modify files (create, delete, rename, move) emit events here.
 * The sidebar file tree listens and refreshes the affected folder(s).
 */

type FileTreeEventType = 'created' | 'deleted' | 'renamed' | 'moved'

interface FileTreeEvent {
  type: FileTreeEventType
  /** The folder that was affected (parent of the file). undefined = root. */
  folderId?: string
  /** For moves: the destination folder. */
  targetFolderId?: string
}

type Listener = (event: FileTreeEvent) => void

const listeners = new Set<Listener>()

export function onFileTreeChange(listener: Listener): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function emitFileTreeChange(event: FileTreeEvent) {
  listeners.forEach((fn) => fn(event))
}
