/**
 * Typed message protocol for Flutter <-> Webview editor communication.
 *
 * Each message has a `type` discriminant. New sprints add new types
 * without changing existing ones. The Flutter side ignores unknown types.
 */

// Flutter -> Webview (host -> editor)
export type HostToEditorMessage =
  | { type: 'setContent'; markdown: string }
  | { type: 'setEditable'; editable: boolean }
  | { type: 'getMarkdown'; requestId: string }
  | { type: 'runCommand'; command: string; payload?: unknown }
  // Editor font scale — 1.0 = default, <1 zooms out, >1 zooms in.
  | { type: 'setZoom'; scale: number }
  // Editor theme. The standalone boots dark; hosts with a light UI send this.
  | { type: 'setTheme'; theme: 'dark' | 'light' }
  // B2: image resolution response from Flutter
  | { type: 'assetResolved'; requestId: string; dataUrl: string }
  | { type: 'assetResolveFailed'; requestId: string; error: string }
  // B4: wiki-link note list for autocomplete
  | { type: 'setNoteList'; notes: Array<{ id: string; name: string }> }
  // In-note find. Plaintext (not regex). Empty query clears highlights.
  | { type: 'find'; query: string; caseSensitive?: boolean }
  | { type: 'findNext' }
  | { type: 'findPrev' }
  | { type: 'clearFind' }

// Webview -> Flutter (editor -> host)
export type EditorToHostMessage =
  | { type: 'ready' }
  | { type: 'contentChanged'; markdown: string; isDirty: boolean }
  | { type: 'saveRequested' }
  | { type: 'markdownResult'; requestId: string; markdown: string }
  | { type: 'error'; message: string }
  // B2: image paste/drop — Flutter handles encrypt + upload
  | { type: 'imagePasted'; requestId: string; base64: string; mimeType: string }
  // B2: asset URL needs resolution — Flutter downloads + decrypts
  | { type: 'resolveAsset'; requestId: string; assetFileId: string }
  // B4: wiki-link clicked
  | { type: 'wikiLinkClicked'; noteTitle: string }
  // B4: parsed wiki-links on save
  | { type: 'linksResolved'; links: Array<{ title: string; fileId?: string }> }
  // In-note find. index is 0-based current match, or -1 if count is 0.
  | { type: 'findResult'; query: string; count: number; index: number }
  // User hit Cmd/Ctrl+F inside the webview — host should show its find bar.
  | { type: 'findRequested' }
