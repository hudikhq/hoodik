/**
 * Milkdown command keys used by the editor toolbar and bridge.
 *
 * These match the command names from @milkdown/preset-commonmark and
 * @milkdown/preset-gfm. Using constants prevents string typos and
 * keeps the web toolbar and Flutter bridge in sync.
 */
export const EditorCommands = {
  // Text formatting
  ToggleStrong: 'ToggleStrong',
  ToggleEmphasis: 'ToggleEmphasis',
  ToggleStrikeThrough: 'ToggleStrikeThrough',
  ToggleLink: 'ToggleLink',

  // Block formatting
  WrapInHeading: 'WrapInHeading',
  WrapInBulletList: 'WrapInBulletList',
  WrapInOrderedList: 'WrapInOrderedList',
  WrapInBlockquote: 'WrapInBlockquote',
  CreateCodeBlock: 'CreateCodeBlock',
  InsertTable: 'InsertTable',

  // History
  Undo: 'Undo',
  Redo: 'Redo',
} as const

export type EditorCommand = typeof EditorCommands[keyof typeof EditorCommands]
