import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

import type { Capabilities, ShareRole } from 'types'

import { setDefaultCipher } from '!/cryptfns/cipher'

import * as api from './api'

/**
 * Kept in its own module rather than in `shares/index.ts`, which imports the
 * storage store. Anything under `!/storage` that needs to read a capability
 * would otherwise close a cycle — storage → shares → storage — and a cycle
 * across module initialization leaves bindings undefined at boot, which the
 * app shows as a loading screen that never resolves.
 *
 * Nothing here reaches for storage, so importing it from either side is safe.
 */
const FAIL_CLOSED_CAPABILITIES: Capabilities = {
  sharing: { enabled: false, roles: [] },
  editable_folders: false,
  share_groups: false,
  audit_log: false,
  fork: false,
  direct_transfer: false
}

/**
 * Capability advertisement, fetched from the public `GET /api/capabilities`
 * endpoint at app boot and on every successful login. Every getter
 * defaults to `false` so a missing or failed fetch fails closed.
 */
export const capabilitiesStore = defineStore('capabilities', () => {
  const caps = ref<Capabilities | null>(null)
  const loading = ref(false)
  const lastFetchedAt = ref<number | null>(null)
  const fetchError = ref<string | null>(null)

  const sharingEnabled = computed<boolean>(() => caps.value?.sharing.enabled === true)

  const roles = computed<ShareRole[]>(() => caps.value?.sharing.roles ?? [])

  const editableFolders = computed<boolean>(
    () => sharingEnabled.value && caps.value?.editable_folders === true
  )

  const shareGroups = computed<boolean>(
    () => sharingEnabled.value && caps.value?.share_groups === true
  )

  const auditLog = computed<boolean>(
    () => sharingEnabled.value && caps.value?.audit_log === true
  )

  const forkEnabled = computed<boolean>(
    () => sharingEnabled.value && caps.value?.fork === true
  )

  /**
   * Unlike the getters above this one does not hang off `sharingEnabled` —
   * how bytes reach the browser has nothing to do with whether sharing is
   * switched on.
   */
  const directTransfer = computed<boolean>(() => caps.value?.direct_transfer === true)

  async function fetch(): Promise<void> {
    loading.value = true
    fetchError.value = null
    try {
      caps.value = await api.getCapabilities()
      setDefaultCipher(caps.value.default_cipher ?? 'aegis128l')
      lastFetchedAt.value = Math.floor(Date.now() / 1000)
      failedAt = null
    } catch (e) {
      caps.value = FAIL_CLOSED_CAPABILITIES
      fetchError.value = 'errors.capabilitiesUnavailable'
      failedAt = Math.floor(Date.now() / 1000)
    } finally {
      loading.value = false
    }
  }

  let inflight: Promise<void> | null = null

  /**
   * How long a failed fetch stands as the answer. The gates sit in the
   * transfer hot path — one per chunk — and a server that fails slowly would
   * otherwise add its whole timeout to every one of them.
   */
  const NEGATIVE_TTL_SECONDS = 30

  let failedAt: number | null = null

  /**
   * Resolve once the advertisement has been fetched at least once, fetching
   * it now if nobody has. The authenticated app fetches at login, but the
   * public link page has no login — and a gate that reads the store without
   * this sees the fail-closed null and quietly disables a capability the
   * server advertises. Concurrent callers share one request; `fetch` installs
   * fail-closed defaults on error, so this always resolves.
   */
  async function ensureFetched(): Promise<void> {
    if (lastFetchedAt.value !== null) return
    if (failedAt !== null && Math.floor(Date.now() / 1000) - failedAt < NEGATIVE_TTL_SECONDS) {
      return
    }
    inflight ??= fetch().finally(() => {
      inflight = null
    })
    return inflight
  }

  function reset(): void {
    caps.value = null
    lastFetchedAt.value = null
    fetchError.value = null
    failedAt = null
  }

  return {
    caps,
    loading,
    lastFetchedAt,
    fetchError,
    sharingEnabled,
    roles,
    editableFolders,
    shareGroups,
    auditLog,
    forkEnabled,
    directTransfer,
    fetch,
    ensureFetched,
    reset
  }
})

export type CapabilitiesStore = ReturnType<typeof capabilitiesStore>
