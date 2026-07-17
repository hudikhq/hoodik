import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'

// A fixed curve keypair, so the card renders and produces a known bundle.
vi.mock('!/crypto', () => ({
  store: () => ({
    keypair: {
      keyType: 'curve25519',
      input: 'ED-PRIV',
      wrappingPrivate: 'X-PRIV',
      legacyPrivate: 'RSA-PRIV',
      publicKey: 'ED-PUB',
      fingerprint: 'fp',
      keySize: 0
    }
  })
}))

import RecoveryKey from '../../src/views/account/index/RecoveryKey.vue'

describe('RecoveryKey card', () => {
  it('reveals the recovery key on click (AppButton needs type=button to emit)', async () => {
    const wrapper = mount(RecoveryKey)

    // Hidden until revealed.
    expect(wrapper.find('pre').exists()).toBe(false)

    const reveal = wrapper.findAll('button').find((b) => b.text().includes('Reveal'))
    expect(reveal).toBeTruthy()
    await reveal!.trigger('click')
    await flushPromises()

    const pre = wrapper.find('pre')
    expect(pre.exists()).toBe(true)
    expect(pre.text()).toContain('ed:ED-PRIV')
    expect(pre.text()).toContain('rsa:RSA-PRIV')
  })
})
