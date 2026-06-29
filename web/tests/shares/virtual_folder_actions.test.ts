import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'

import ActionsButtons from '../../src/components/files/browser/ActionsButtons.vue'
import { capabilitiesStore } from '../../services/shares'
import * as sharesApi from '../../services/shares/api'

import type { AppFile, Capabilities } from '../../types'

const FILE_ID = '11111111-1111-1111-1111-111111111111'
const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'

function ownedFile(overrides: Partial<AppFile> = {}): AppFile {
  return {
    id: FILE_ID,
    user_id: OWNER_ID,
    is_owner: true,
    name: 'doc.pdf',
    name_hash: '',
    mime: 'application/pdf',
    chunks: 1,
    chunks_stored: 1,
    finished_upload_at: 1_700_000_500,
    file_id: null,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: 'aegis128l',
    ...overrides
  } as unknown as AppFile
}

function sharedFile(role: 'reader' | 'editor' | 'co-owner', overrides: Partial<AppFile> = {}): AppFile {
  return ownedFile({
    is_owner: false,
    share_role: role,
    shared_by_email: 'alice@example.com',
    owner_email: 'alice@example.com',
    ...overrides
  })
}

async function withCapabilities(overrides: Partial<Capabilities> = {}): Promise<void> {
  const caps: Capabilities = {
    sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
    editable_folders: true,
    share_groups: true,
    audit_log: true,
    fork: true,
    ...overrides
  }
  vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue(caps)
  await capabilitiesStore().fetch()
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('Virtual folder row actions: role gating', () => {
  it('reader_row_shows_sharing_for_inspection_but_no_fork_or_delete', async () => {
    await withCapabilities()
    const wrapper = mount(ActionsButtons, {
      props: { modelValue: sharedFile('reader') }
    })
    expect(wrapper.find('[data-testid="actions-fork"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="actions-leave"]').exists()).toBe(true)
    // Reader can inspect the roster — the modal opens read-only.
    expect(wrapper.find('[data-testid="actions-share-account"]').exists()).toBe(true)
    expect(wrapper.find('[name="delete"]').exists()).toBe(false)
  })

  it('editor_row_shows_sharing_for_inspection_but_no_fork', async () => {
    await withCapabilities()
    const wrapper = mount(ActionsButtons, {
      props: { modelValue: sharedFile('editor') }
    })
    expect(wrapper.find('[data-testid="actions-fork"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="actions-leave"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="actions-share-account"]').exists()).toBe(true)
    expect(wrapper.find('[name="delete"]').exists()).toBe(false)
  })

  it('co_owner_row_offers_fork_leave_and_reshare', async () => {
    await withCapabilities()
    const wrapper = mount(ActionsButtons, {
      props: { modelValue: sharedFile('co-owner') }
    })
    expect(wrapper.find('[data-testid="actions-fork"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="actions-leave"]').exists()).toBe(true)
    // Co-owner can reshare — the Sharing entry is visible and the
    // SharingModal handles the disable-co-owner gating internally.
    expect(wrapper.find('[data-testid="actions-share-account"]').exists()).toBe(true)
    expect(wrapper.find('[name="delete"]').exists()).toBe(false)
  })

  it('owned_row_hides_fork_and_leave_keeps_delete_and_sharing', async () => {
    await withCapabilities()
    const wrapper = mount(ActionsButtons, {
      props: { modelValue: ownedFile() }
    })
    expect(wrapper.find('[data-testid="actions-fork"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="actions-leave"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="actions-share-account"]').exists()).toBe(true)
    expect(wrapper.find('[name="delete"]').exists()).toBe(true)
  })

  it('fork_button_hidden_when_fork_capability_disabled', async () => {
    await withCapabilities({ fork: false })
    const wrapper = mount(ActionsButtons, {
      props: { modelValue: sharedFile('co-owner') }
    })
    expect(wrapper.find('[data-testid="actions-fork"]').exists()).toBe(false)
    // Leave and Sharing still render — those are not capability-gated
    // beyond the umbrella `sharing.enabled` flag.
    expect(wrapper.find('[data-testid="actions-leave"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="actions-share-account"]').exists()).toBe(true)
  })

  it('fork_click_emits_fork_event_with_file_payload', async () => {
    await withCapabilities()
    const file = sharedFile('co-owner')
    const wrapper = mount(ActionsButtons, { props: { modelValue: file } })
    await wrapper.get('[data-testid="actions-fork"]').trigger('click')
    expect(wrapper.emitted('fork')?.[0]?.[0]).toEqual(file)
  })

  it('leave_click_emits_leave_event_with_file_payload', async () => {
    await withCapabilities()
    const file = sharedFile('reader')
    const wrapper = mount(ActionsButtons, { props: { modelValue: file } })
    await wrapper.get('[data-testid="actions-leave"]').trigger('click')
    expect(wrapper.emitted('leave')?.[0]?.[0]).toEqual(file)
  })

  it('leave_label_is_short_for_dropdown_width', async () => {
    // The dropdown surface is narrow — the long "Remove from my drive"
    // label wraps and reads as a sentence rather than a verb. The
    // follow-up confirm modal carries the disambiguating "X still owns
    // this file" context, so a single verb fits the action's role here.
    await withCapabilities()
    const wrapper = mount(ActionsButtons, {
      props: { modelValue: sharedFile('reader') }
    })
    const btn = wrapper.get('[data-testid="actions-leave"]')
    expect(btn.text()).toBe('Remove')
  })

  /**
   * The synthetic "Shared with me" folder is a client-side injection — its
   * id never makes it past the SPA. Every action below interpolates the
   * row's id into a real backend endpoint, so the dropdown renders empty
   * for the synthetic row to avoid 400 toasts on the `__shared_with_me__`
   * literal.
   */
  it('synthetic_shared_with_me_row_hides_every_action', async () => {
    await withCapabilities()
    const synthetic: AppFile = {
      id: '__shared_with_me__',
      user_id: '',
      is_owner: false,
      name: 'Shared with me',
      name_hash: '',
      mime: 'dir',
      chunks: 0,
      file_id: null,
      file_modified_at: 0,
      created_at: 0,
      is_new: false,
      editable: false,
      active_version: 1,
      encrypted_key: '',
      encrypted_name: '',
      cipher: ''
    } as unknown as AppFile
    const wrapper = mount(ActionsButtons, { props: { modelValue: synthetic } })
    expect(wrapper.find('[name="details"]').exists()).toBe(false)
    expect(wrapper.find('[name="rename"]').exists()).toBe(false)
    expect(wrapper.find('[name="delete"]').exists()).toBe(false)
    expect(wrapper.find('[name="download"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="actions-share-account"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="actions-fork"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="actions-leave"]').exists()).toBe(false)
  })
})
