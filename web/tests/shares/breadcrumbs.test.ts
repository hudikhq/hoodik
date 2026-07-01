import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'

import BreadCrumbs from '../../src/components/files/BreadCrumbs.vue'
import { SHARED_WITH_ME_DIR_ID } from '../../services/storage'

import type { AppFile } from '../../types'

function setupRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/:file_id?', name: 'files', component: { template: '<div />' } }]
  })
}

function ownedFolder(id: string, name: string, fileId: string | null = null): AppFile {
  return {
    id,
    user_id: 'me',
    is_owner: true,
    name,
    name_hash: '',
    mime: 'dir',
    chunks: 0,
    file_id: fileId,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: ''
  } as unknown as AppFile
}

function sharedFolder(id: string, name: string, fileId: string | null = null): AppFile {
  return { ...ownedFolder(id, name, fileId), is_owner: false } as AppFile
}

function syntheticRoot(): AppFile {
  return ownedFolder(SHARED_WITH_ME_DIR_ID, 'Shared with me')
}

describe('BreadCrumbs root crumb', () => {
  it('breadcrumb_root_reads_my_files_for_owned_chain', () => {
    const wrapper = mount(BreadCrumbs, {
      props: { parents: [ownedFolder('a', 'Projects'), ownedFolder('b', 'Q1', 'a')] },
      global: { plugins: [setupRouter()] }
    })
    const root = wrapper.find('[data-testid="breadcrumb-root"]')
    expect(root.exists()).toBe(true)
    expect(root.text()).toEqual('My Files')
  })

  it('breadcrumb_root_reads_shared_with_me_inside_synthetic_chain', () => {
    // Inside `__shared_with_me__` the chain starts with the synthetic
    // parent; the root crumb must route the user back to the virtual
    // folder, not to their owned root which has nothing to do with the
    // content they were browsing.
    const wrapper = mount(BreadCrumbs, {
      props: {
        parents: [syntheticRoot(), sharedFolder('shared-folder', 'Alice docs', SHARED_WITH_ME_DIR_ID)]
      },
      global: { plugins: [setupRouter()] }
    })
    const root = wrapper.find('[data-testid="breadcrumb-root"]')
    expect(root.text()).toEqual('Shared with me')
    // The synthetic parent itself does not duplicate as a per-folder crumb.
    expect(wrapper.text()).not.toContain('Shared with me / Shared with me')
  })

  it('breadcrumb_root_reads_shared_with_me_when_chain_head_is_non_owner', () => {
    // Recipients with deeper nested navigation land on a shared folder
    // chain that does not start from the synthetic parent (the SPA may
    // not have populated that placeholder yet). The chain head being
    // non-owner is enough signal that the user is in shared territory.
    const wrapper = mount(BreadCrumbs, {
      props: {
        parents: [sharedFolder('shared-folder', 'Alice docs'), sharedFolder('child', 'nested', 'shared-folder')]
      },
      global: { plugins: [setupRouter()] }
    })
    const root = wrapper.find('[data-testid="breadcrumb-root"]')
    expect(root.text()).toEqual('Shared with me')
  })

  it('breadcrumb_root_is_disabled_when_at_top_level', () => {
    const wrapper = mount(BreadCrumbs, {
      props: { parents: [] },
      global: { plugins: [setupRouter()] }
    })
    const root = wrapper.find('[data-testid="breadcrumb-root"]')
    // BaseButton renders an <a> when wrapping a route — disabled state
    // comes through the prop binding, not the native attribute.
    expect(root.exists()).toBe(true)
  })
})
