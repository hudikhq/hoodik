import { describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'

import FolderPicker from '../src/components/ui/FolderPicker.vue'
import * as meta from '../services/storage/meta'

import type { KeyPair } from '../types'

describe('FolderPicker without key material', () => {
  it('mounts quietly for a guest instead of decrypting with a null key', async () => {
    // A public-link viewer has an empty keypair; the picker used to push
    // null into the wasm decrypt and throw from its mounted hook.
    const find = vi.spyOn(meta, 'find')

    const wrapper = mount(FolderPicker, {
      props: {
        keypair: { publicKey: null, input: null, fingerprint: null } as unknown as KeyPair
      }
    })
    await flushPromises()

    expect(find).not.toHaveBeenCalled()
    expect(wrapper.text()).not.toContain('Loading')

    find.mockRestore()
    wrapper.unmount()
  })
})
