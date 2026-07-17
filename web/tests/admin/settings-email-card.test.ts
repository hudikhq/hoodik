import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'

vi.mock('!/admin/settings', () => ({ index: vi.fn(), update: vi.fn() }))

import { index } from '!/admin/settings'
import SettingsInner from '../../src/views/admin/settings/SettingsInner.vue'

const mockedIndex = vi.mocked(index)

function settings(mailer_disable_test?: boolean) {
  return {
    users: {
      allow_register: true,
      enforce_email_activation: false,
      email_whitelist: { rules: [] },
      email_blacklist: { rules: [] }
    },
    sharing: { enabled: true, default_cipher: 'aegis256' },
    mailer_disable_test
  }
}

// SettingsInner has a top-level `await init()`, so it needs a Suspense parent to
// resolve before assertions. The two settings cards are stubbed — this test is
// about SettingsInner's decision to render the email card, not the cards.
async function mountInner() {
  const Wrapper = {
    components: { SettingsInner },
    template: '<Suspense><SettingsInner /></Suspense>'
  }
  const wrapper = mount(Wrapper, {
    global: {
      stubs: {
        UserSettings: { template: '<div class="user-settings-stub" />' },
        EmailSettings: { template: '<div class="email-settings-stub" />' }
      }
    }
  })
  await flushPromises()
  return wrapper
}

describe('SettingsInner — test-email card visibility', () => {
  beforeEach(() => mockedIndex.mockReset())

  it('shows the email card when the mailer test is not disabled', async () => {
    mockedIndex.mockResolvedValue(settings(false))
    const wrapper = await mountInner()
    expect(wrapper.find('.email-settings-stub').exists()).toBe(true)
  })

  it('hides the email card when MAILER_DISABLE_TEST is set', async () => {
    mockedIndex.mockResolvedValue(settings(true))
    const wrapper = await mountInner()
    expect(wrapper.find('.email-settings-stub').exists()).toBe(false)
  })
})
