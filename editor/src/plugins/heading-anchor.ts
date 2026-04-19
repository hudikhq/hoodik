/**
 * ProseMirror plugin that adds id attributes to heading nodes
 * so that in-document anchor links (#heading-text) have a target to scroll to.
 */
import type { MilkdownPlugin, Ctx } from '@milkdown/ctx'
import { InitReady, prosePluginsCtx } from '@milkdown/core'
import { Plugin, PluginKey } from '@milkdown/prose/state'
import type { Node } from '@milkdown/prose/model'
import { Decoration, DecorationSet } from '@milkdown/prose/view'

function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^\w\s-]/g, '')
    .replace(/[\s_]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function textContent(node: Node): string {
  let text = ''
  node.forEach((child) => {
    if (child.isText) text += child.text
    else text += textContent(child)
  })
  return text
}

const headingAnchorKey = new PluginKey('heading-anchor')

export function createHeadingAnchorPlugin(): MilkdownPlugin {
  const plugin: MilkdownPlugin = (ctx: Ctx) => {
    return async () => {
      await ctx.wait(InitReady)

      const prosePlugin = new Plugin({
        key: headingAnchorKey,
        props: {
          decorations(state) {
            const decorations: Decoration[] = []

            state.doc.descendants((node, pos) => {
              if (node.type.name === 'heading') {
                const text = textContent(node)
                const id = slugify(text)
                if (id) {
                  decorations.push(
                    Decoration.node(pos, pos + node.nodeSize, { id })
                  )
                }
              }
            })

            return DecorationSet.create(state.doc, decorations)
          }
        }
      })

      ctx.update(prosePluginsCtx, (plugins) => [...plugins, prosePlugin])
    }
  }

  plugin.meta = {
    package: '@hoodik/heading-anchor',
    displayName: 'Heading Anchor IDs'
  }

  return plugin
}
