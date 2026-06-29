import { describe, expect, it, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'

import AsideMenuItem from '../../src/components/ui/AsideMenuItem.vue'

function setupRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/share', name: 'share', component: { template: '<div />' } }]
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
})

describe('AsideMenuItem: Share entry', () => {
  it('aside_menu_share_entry_renders_without_badge', () => {
    // The numeric unread badge used to wedge in next to the Share label.
    // That counter is still useful inside the Share hub (header pill), but
    // the sidebar shouldn't carry a parallel signal — recipients act on
    // shared content through the virtual folder under /files, not by
    // navigating to /share/public.
    const wrapper = mount(AsideMenuItem, {
      props: {
        item: { to: { name: 'share' }, icon: 'M1,1', label: 'Share' }
      },
      global: { plugins: [setupRouter()] }
    })

    expect(wrapper.text()).toContain('Share')
    expect(wrapper.find('[data-testid="aside-menu-share-badge"]').exists()).toBe(false)
  })
})
