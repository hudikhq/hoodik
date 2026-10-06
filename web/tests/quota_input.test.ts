import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'

import QuotaInput from '../src/components/ui/QuotaInput.vue'

const GB = 1024 ** 3
const TB = 1024 ** 4

function amountInput(wrapper: ReturnType<typeof mount>) {
  return wrapper.find('[data-testid="quota-amount-input"]').element as HTMLInputElement
}

function unitSelect(wrapper: ReturnType<typeof mount>) {
  return wrapper.find('[data-testid="quota-unit-select"]')
}

function lastEmitted(wrapper: ReturnType<typeof mount>) {
  const events = wrapper.emitted('update:modelValue') || []
  return events[events.length - 1]?.[0]
}

describe('QuotaInput', () => {
  it('shows an existing quota in the largest fitting unit', () => {
    const wrapper = mount(QuotaInput, { props: { modelValue: 2 * TB } })
    expect(amountInput(wrapper).value).toBe('2')
    expect((unitSelect(wrapper).element as HTMLSelectElement).value).toBe('TB')
  })

  it('emits bytes for an exact amount above 1024 GB', async () => {
    const wrapper = mount(QuotaInput, { props: { modelValue: 0 } })
    await wrapper.find('[data-testid="quota-amount-input"]').setValue('1500')
    expect(lastEmitted(wrapper)).toBe(1500 * GB)
  })

  it('recomputes bytes when the unit changes', async () => {
    const wrapper = mount(QuotaInput, { props: { modelValue: 3 * GB } })
    await unitSelect(wrapper).setValue('TB')
    expect(lastEmitted(wrapper)).toBe(3 * TB)
  })

  it('disables the inputs while the default quota is used', () => {
    const wrapper = mount(QuotaInput, { props: { modelValue: undefined } })
    expect(amountInput(wrapper).disabled).toBe(true)
    expect((unitSelect(wrapper).element as HTMLSelectElement).disabled).toBe(true)
  })
})
