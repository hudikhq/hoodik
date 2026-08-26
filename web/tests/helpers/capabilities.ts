import { capabilitiesStore } from '../../services/shares/capabilities'
import type { Capabilities } from '../../types/shares'

/**
 * Put the capability store into a known state, marked as fetched.
 *
 * Setting only `caps` is not enough: the direct-transfer gates await
 * `ensureFetched`, and a store with no `lastFetchedAt` issues a real request —
 * which lands in whatever fetch stub the test installed (breaking its request
 * counting) or fails in jsdom and stomps the fixture with the fail-closed set.
 */
export function installCapabilities(overrides: Partial<Capabilities> = {}): void {
  const store = capabilitiesStore()
  store.caps = {
    sharing: { enabled: false, roles: [] },
    editable_folders: false,
    share_groups: false,
    audit_log: false,
    fork: false,
    direct_transfer: false,
    ...overrides
  }
  store.lastFetchedAt = Math.floor(Date.now() / 1000)
}
