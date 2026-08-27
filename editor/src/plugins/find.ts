/**
 * ProseMirror plugin that highlights plaintext in-note search matches
 * and keeps a current-match index the host can step through.
 */
import type { MilkdownPlugin, Ctx } from '@milkdown/ctx'
import { InitReady, prosePluginsCtx } from '@milkdown/core'
import { Plugin, PluginKey } from '@milkdown/prose/state'
import type { Node } from '@milkdown/prose/model'
import type { EditorView } from '@milkdown/prose/view'
import { Decoration, DecorationSet } from '@milkdown/prose/view'
import { findMatchOffsets, type TextMatch } from './find-matches'

export type FindMeta =
  | { type: 'query'; query: string; caseSensitive: boolean }
  | { type: 'next' }
  | { type: 'prev' }
  | { type: 'clear' }

export interface FindPluginState {
  query: string
  caseSensitive: boolean
  matches: TextMatch[]
  /** 0-based current match, or -1 if there are no matches. */
  index: number
}

export const findPluginKey = new PluginKey<FindPluginState>('hoodik-find')

export const emptyFindState: FindPluginState = {
  query: '',
  caseSensitive: false,
  matches: [],
  index: -1,
}

/**
 * Search each textblock's visible text so matches can span mark boundaries
 * (bold/italic/code) without treating the query as a regex.
 */
export function findMatchesInDoc(
  doc: Node,
  query: string,
  caseSensitive: boolean,
): TextMatch[] {
  if (!query) return []

  const matches: TextMatch[] = []
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return

    const contentStart = pos + 1
    let text = ''
    const indexToPos: number[] = []

    node.descendants((child, childPos) => {
      if (!child.isText || !child.text) return
      const abs = contentStart + childPos
      for (let i = 0; i < child.text.length; i++) {
        indexToPos.push(abs + i)
        text += child.text[i]
      }
    })

    for (const m of findMatchOffsets(text, query, caseSensitive)) {
      const from = indexToPos[m.from]
      const last = indexToPos[m.to - 1]
      if (from === undefined || last === undefined) continue
      matches.push({ from, to: last + 1 })
    }
  })
  return matches
}

function wrapIndex(index: number, count: number, delta: number): number {
  if (count === 0) return -1
  const start = index < 0 ? (delta > 0 ? -1 : 0) : index
  return (start + delta + count) % count
}

function computeState(
  doc: Node,
  query: string,
  caseSensitive: boolean,
  preferredIndex: number,
): FindPluginState {
  if (!query) return { ...emptyFindState, caseSensitive }
  const matches = findMatchesInDoc(doc, query, caseSensitive)
  if (matches.length === 0) {
    return { query, caseSensitive, matches, index: -1 }
  }
  const index = Math.min(Math.max(preferredIndex, 0), matches.length - 1)
  return { query, caseSensitive, matches, index }
}

function applyMeta(
  meta: FindMeta,
  value: FindPluginState,
  doc: Node,
): FindPluginState {
  switch (meta.type) {
    case 'query':
      return computeState(doc, meta.query, meta.caseSensitive, 0)
    case 'clear':
      return emptyFindState
    case 'next':
      if (!value.query) return emptyFindState
      return {
        ...value,
        index: wrapIndex(value.index, value.matches.length, 1),
      }
    case 'prev':
      if (!value.query) return emptyFindState
      return {
        ...value,
        index: wrapIndex(value.index, value.matches.length, -1),
      }
  }
}

export function getFindState(view: EditorView): FindPluginState {
  return findPluginKey.getState(view.state) ?? emptyFindState
}

export function dispatchFind(view: EditorView, meta: FindMeta): FindPluginState {
  const tr = view.state.tr.setMeta(findPluginKey, meta).setMeta('addToHistory', false)
  view.dispatch(tr)
  return getFindState(view)
}

export function scrollToCurrentMatch(view: EditorView): void {
  requestAnimationFrame(() => {
    view.dom
      .querySelector('.hoodik-find-match-current')
      ?.scrollIntoView({ block: 'center', inline: 'nearest' })
  })
}

export function createFindPlugin(): MilkdownPlugin {
  const plugin: MilkdownPlugin = (ctx: Ctx) => {
    return async () => {
      await ctx.wait(InitReady)

      const prosePlugin = new Plugin<FindPluginState>({
        key: findPluginKey,
        state: {
          init: () => emptyFindState,
          apply(tr, value, _oldState, newState) {
            const meta = tr.getMeta(findPluginKey) as FindMeta | undefined
            if (meta) return applyMeta(meta, value, newState.doc)
            if (tr.docChanged && value.query) {
              return computeState(
                newState.doc,
                value.query,
                value.caseSensitive,
                value.index,
              )
            }
            return value
          },
        },
        props: {
          decorations(state) {
            const find = findPluginKey.getState(state)
            if (!find || find.matches.length === 0) return DecorationSet.empty
            const decorations = find.matches.map((m, i) =>
              Decoration.inline(m.from, m.to, {
                class:
                  i === find.index
                    ? 'hoodik-find-match hoodik-find-match-current'
                    : 'hoodik-find-match',
              }),
            )
            return DecorationSet.create(state.doc, decorations)
          },
        },
      })

      ctx.update(prosePluginsCtx, (plugins) => [...plugins, prosePlugin])
    }
  }

  plugin.meta = {
    package: '@hoodik/find',
    displayName: 'Find in note',
  }

  return plugin
}
