import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'

import * as sharesApi from '../../services/shares/api'
import SharingPreferences from '../../src/views/account/index/SharingPreferences.vue'

import type { User } from '../../types'

function makeUser(overrides: Partial<User> = {}): User {
  return {
    id: '11111111-1111-1111-1111-111111111111',
    email: 'alice@example.com',
    pubkey: '',
    fingerprint: '',
    created_at: 1_700_000_000,
    updated_at: 1_700_000_000,
    secret: false,
    share_notifications_enabled: true,
    ...overrides
  }
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('account settings: sharing notifications', () => {
  it('share_notifications_toggle_default_on', async () => {
    const wrapper = mount(SharingPreferences, { props: { user: makeUser() } })
    await flushPromises()
    const input = wrapper.get('[data-testid="account-share-notifications-toggle"]')
    expect((input.element as HTMLInputElement).checked).toBe(true)
  })

  it('share_notifications_toggle_persists_via_patch_me', async () => {
    const spy = vi
      .spyOn(sharesApi, 'patchMe')
      .mockResolvedValue({ id: '1', share_notifications_enabled: false })
    const wrapper = mount(SharingPreferences, { props: { user: makeUser() } })
    await flushPromises()
    await wrapper.get('[data-testid="account-share-notifications-toggle"]').trigger('change')
    await flushPromises()
    expect(spy).toHaveBeenCalledWith({ share_notifications_enabled: false })
    expect(wrapper.get('[data-testid="account-share-notifications-label"]').text()).toContain(
      'off'
    )
  })
})
