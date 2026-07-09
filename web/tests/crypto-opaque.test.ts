import { describe, it, expect } from 'vitest'
import * as crypto from '../services/cryptfns'

/**
 * A full OPAQUE round-trip needs the server half (registration/login response
 * generation), which lives on the server and is not exposed in WASM. These
 * tests only cover what the client can produce on its own: well-formed start
 * messages and states.
 */
describe('OPAQUE test', () => {
  it('UNIT: OPAQUE: registration start produces state and message', async () => {
    const { state, message } = await crypto.opaque.clientRegistrationStart('correct horse')

    expect(state.length).toBeGreaterThan(0)
    expect(message.length).toBeGreaterThan(0)
  })

  it('UNIT: OPAQUE: login start produces state and message', async () => {
    const { state, message } = await crypto.opaque.clientLoginStart('correct horse')

    expect(state.length).toBeGreaterThan(0)
    expect(message.length).toBeGreaterThan(0)
  })

  it('UNIT: OPAQUE: registration start is randomized per call', async () => {
    const first = await crypto.opaque.clientRegistrationStart('correct horse')
    const second = await crypto.opaque.clientRegistrationStart('correct horse')

    expect(first.message).not.toBe(second.message)
  })
})
