import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'

import MoveIntoSharedConfirmModal from '../../src/components/files/modals/MoveIntoSharedConfirmModal.vue'

beforeEach(() => {
  setActivePinia(createPinia())
})

function mountModal(props: Record<string, unknown> = {}) {
  return mount(MoveIntoSharedConfirmModal, {
    props: {
      modelValue: true,
      folderName: 'Reports',
      destinationName: "Bob's shared",
      itemCount: 4,
      memberLabels: ['bob@example.com', 'carol@example.com'],
      progress: null,
      ...props
    }
  })
}

describe('MoveIntoSharedConfirmModal', () => {
  it('renders the folder, destination, item count, and members', () => {
    const wrapper = mountModal()
    const text = wrapper.get('[data-testid="move-share-confirm-message"]').text()
    expect(text).toContain('Reports')
    expect(text).toContain("Bob's shared")
    expect(text).toContain('4 items')
    expect(text).toContain('bob@example.com, carol@example.com')
  })

  it('summarizes a single item without pluralizing', () => {
    const wrapper = mountModal({ itemCount: 1, memberLabels: ['bob@example.com'] })
    expect(wrapper.get('[data-testid="move-share-confirm-message"]').text()).toContain('1 item ')
  })

  it('truncates a long member list', () => {
    const wrapper = mountModal({
      memberLabels: ['a@x.com', 'b@x.com', 'c@x.com', 'd@x.com', 'e@x.com']
    })
    expect(wrapper.get('[data-testid="move-share-confirm-message"]').text()).toContain(
      'a@x.com, b@x.com, c@x.com and 2 more'
    )
  })

  it('shows a determinate progress bar once progress is reported', async () => {
    const wrapper = mountModal({ progress: 0.5 })
    const bar = wrapper.get('[data-testid="move-share-confirm-progress"]')
    expect(bar.text()).toContain('50%')
  })

  it('emits confirm when the primary button is clicked', async () => {
    const wrapper = mountModal()
    const buttons = wrapper.findAll('button')
    const moveBtn = buttons.find((b) => b.text().includes('Move and share'))
    await moveBtn?.trigger('click')
    expect(wrapper.emitted('confirm')).toBeTruthy()
  })

  it('emits cancel when the cancel button is clicked', async () => {
    const wrapper = mountModal()
    const buttons = wrapper.findAll('button')
    const cancelBtn = buttons.find((b) => b.text().trim() === 'Cancel')
    await cancelBtn?.trigger('click')
    expect(wrapper.emitted('cancel')).toBeTruthy()
  })
})
