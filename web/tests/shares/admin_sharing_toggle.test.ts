import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'

import UserSettings from '../../src/views/admin/settings/UserSettings.vue'
import type { Data } from '../../types/admin/settings'

vi.mock('@/components/ui/CardBox.vue', () => ({
  default: { template: '<div><slot /></div>' }
}))
vi.mock('@/components/ui/UniversalCheckbox.vue', () => ({
  default: {
    props: ['modelValue', 'label', 'name', 'disabled'],
    inheritAttrs: false,
    template: `
      <label>
        <input
          type="checkbox"
          :checked="modelValue"
          :disabled="disabled"
          v-bind="$attrs"
          @change="$emit('update:modelValue', $event.target.checked)"
        />
        {{ label }}
      </label>
    `
  }
}))
vi.mock('@/components/ui/BaseIcon.vue', () => ({
  default: { template: '<span />' }
}))
vi.mock('@/components/ui/ListInput.vue', () => ({
  default: { template: '<div />' }
}))
vi.mock('@/components/ui/QuotaSlider.vue', () => ({
  default: { template: '<div />' }
}))
vi.mock('@/components/ui/BaseButton.vue', () => ({
  default: {
    props: ['label', 'icon', 'disabled', 'color'],
    template: '<button :disabled="disabled" @click="$emit(\'click\')">{{ label }}</button>'
  }
}))

function buildSettings(overrides: Partial<Data> = {}): Data {
  return {
    users: {
      allow_register: true,
      enforce_email_activation: true,
      email_whitelist: { rules: [] },
      email_blacklist: { rules: [] }
    },
    sharing: { enabled: true },
    ...overrides
  } as Data
}

beforeEach(() => {
  vi.restoreAllMocks()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('Admin UserSettings: sharing kill switch (B16)', () => {
  it('renders the sharing-enabled toggle bound to settings.sharing.enabled', () => {
    const wrapper = mount(UserSettings, {
      props: {
        modelValue: buildSettings(),
        loading: false
      }
    })
    const toggle = wrapper.find('[data-testid="admin-sharing-enabled-toggle"]')
    expect(toggle.exists()).toBe(true)
    expect((toggle.element as HTMLInputElement).checked).toBe(true)
  })

  it('reflects sharing.enabled=false in the toggle state', () => {
    const wrapper = mount(UserSettings, {
      props: {
        modelValue: buildSettings({ sharing: { enabled: false } }),
        loading: false
      }
    })
    const toggle = wrapper.find('[data-testid="admin-sharing-enabled-toggle"]')
    expect((toggle.element as HTMLInputElement).checked).toBe(false)
  })

  it('user click mutates settings.sharing.enabled via the v-model binding', async () => {
    // UserSettings binds v-model directly through a computed get/set
    // forwarded to the parent's prop, so the v-model write lands as a
    // nested-object mutation on the prop itself (Vue allows it for
    // deep-reactive objects). The test asserts on the source state
    // because that's where SettingsInner reads back to PUT.
    const settings = buildSettings()
    const wrapper = mount(UserSettings, {
      props: {
        modelValue: settings,
        loading: false
      }
    })
    expect(settings.sharing.enabled).toBe(true)
    const toggle = wrapper.find('[data-testid="admin-sharing-enabled-toggle"]')
    await toggle.setValue(false)
    expect(settings.sharing.enabled).toBe(false)
  })

  it('disables the toggle while save is in flight', () => {
    const wrapper = mount(UserSettings, {
      props: {
        modelValue: buildSettings(),
        loading: true
      }
    })
    const toggle = wrapper.find('[data-testid="admin-sharing-enabled-toggle"]')
    expect((toggle.element as HTMLInputElement).disabled).toBe(true)
  })
})
