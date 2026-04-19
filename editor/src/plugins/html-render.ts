import { $view } from '@milkdown/utils'
import { htmlSchema } from '@milkdown/preset-commonmark'
import type { NodeViewConstructor } from '@milkdown/prose/view'
import DOMPurify from 'dompurify'

const SANITIZE_CONFIG = {
  ADD_TAGS: ['img'],
  ADD_ATTR: ['align', 'alt', 'src', 'href', 'target', 'rel', 'label']
}

export const htmlRenderView = $view(
  htmlSchema.node,
  (): NodeViewConstructor => {
    return (node) => {
      const dom = document.createElement('span')
      dom.classList.add('milkdown-html-render')
      dom.contentEditable = 'false'
      dom.innerHTML = DOMPurify.sanitize(node.attrs.value, SANITIZE_CONFIG)

      return {
        dom,
        update: (updatedNode) => {
          if (updatedNode.type.name !== 'html') return false
          dom.innerHTML = DOMPurify.sanitize(updatedNode.attrs.value, SANITIZE_CONFIG)
          return true
        },
        stopEvent: () => true
      }
    }
  }
)
