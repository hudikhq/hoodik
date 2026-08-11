import { $command, $view } from '@milkdown/utils'
import { commandsCtx } from '@milkdown/core'
import { listItemSchema, wrapInBulletListCommand } from '@milkdown/preset-commonmark'
import type { Node } from '@milkdown/prose/model'
import type { EditorState } from '@milkdown/prose/state'
import type { NodeViewConstructor } from '@milkdown/prose/view'

/**
 * Interactive GFM task lists.
 *
 * The gfm preset parses `- [ ]` into a list item with a `checked` attr and
 * serializes it back, but renders only a bare `li[data-item-type="task"]` —
 * no checkbox, nothing to click. This NodeView renders a real
 * `<input type="checkbox">` so tasks can be toggled by mouse or keyboard,
 * and writes the toggle back into the document so it saves as `[x]`.
 *
 * Regular list items pass through with the same DOM the schema would
 * produce; the view only adds structure for items where `checked` is set.
 */
export const taskListItemView = $view(
  listItemSchema.node,
  (): NodeViewConstructor => {
    return (initialNode, view, getPos) => {
      const syncDataset = (dom: HTMLElement, node: Node) => {
        if (node.attrs.label != null) dom.dataset.label = node.attrs.label
        if (node.attrs.listType != null) dom.dataset.listType = node.attrs.listType
        if (node.attrs.spread != null) dom.dataset.spread = node.attrs.spread
      }

      if (initialNode.attrs.checked == null) {
        const dom = document.createElement('li')
        syncDataset(dom, initialNode)

        return {
          dom,
          contentDOM: dom,
          update: (node) => {
            if (node.type !== initialNode.type || node.attrs.checked != null) return false
            syncDataset(dom, node)
            return true
          }
        }
      }

      const dom = document.createElement('li')
      const checkbox = document.createElement('input')
      const contentDOM = document.createElement('div')

      dom.dataset.itemType = 'task'
      checkbox.type = 'checkbox'
      checkbox.classList.add('task-checkbox')
      checkbox.contentEditable = 'false'
      contentDOM.classList.add('task-content')
      dom.append(checkbox, contentDOM)

      const sync = (node: Node) => {
        const checked = Boolean(node.attrs.checked)
        dom.dataset.checked = String(checked)
        checkbox.checked = checked
        syncDataset(dom, node)
      }
      sync(initialNode)

      // Keep the editor selection where it is — a checkbox click is a state
      // toggle, not a caret move.
      checkbox.addEventListener('mousedown', (event) => event.preventDefault())

      // `click` also fires for keyboard activation (Space on the focused
      // input). The browser has already flipped `checked` by the time this
      // runs; cancelling the event would make it restore the old value
      // after the handler and clobber the state the document just took, so
      // the native toggle is left alone and the document follows it. Only a
      // read-only view cancels the toggle.
      checkbox.addEventListener('click', (event) => {
        if (!view.editable) {
          event.preventDefault()
          return
        }

        const pos = getPos()
        if (pos == null) return

        const current = view.state.doc.nodeAt(pos)
        if (!current) return

        view.dispatch(
          view.state.tr.setNodeMarkup(pos, undefined, {
            ...current.attrs,
            checked: !current.attrs.checked
          })
        )
      })

      return {
        dom,
        contentDOM,
        update: (node) => {
          if (node.type !== initialNode.type || node.attrs.checked == null) return false
          sync(node)
          return true
        },
        ignoreMutation: (mutation) => !contentDOM.contains(mutation.target),
        stopEvent: (event) => event.target === checkbox
      }
    }
  }
)

/**
 * Finds the closest ancestor list item around the selection and returns
 * its position, or null when the selection is not inside a list.
 */
function findListItem(state: EditorState): { pos: number; node: Node } | null {
  const { $from } = state.selection
  for (let depth = $from.depth; depth > 0; depth--) {
    const node = $from.node(depth)
    if (node.type.name === 'list_item') {
      return { pos: $from.before(depth), node }
    }
  }
  return null
}

/**
 * Turns the current block into a task-list item, or a task item back into
 * a plain list item. Outside a list the selection is wrapped in a bullet
 * list first, so a single toolbar press works from an ordinary paragraph.
 */
export const toggleTaskListCommand = $command('ToggleTaskList', (ctx) => () => {
  return (state, dispatch) => {
    const found = findListItem(state)

    if (!found) {
      const commands = ctx.get(commandsCtx)
      if (!commands.call(wrapInBulletListCommand.key)) return false
      // Re-enter by key: the selection now sits inside the fresh list item.
      // (The literal avoids a self-referential type in the command const.)
      return commands.call('ToggleTaskList')
    }

    if (dispatch) {
      dispatch(
        state.tr.setNodeMarkup(found.pos, undefined, {
          ...found.node.attrs,
          checked: found.node.attrs.checked == null ? false : null
        })
      )
    }

    return true
  }
})
