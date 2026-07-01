import { onMounted, computed, type ComputedRef } from 'vue'
import { capabilitiesStore } from '!/shares'

/**
 * Reactive wrapper around the public capability advertisement. Initial
 * `fetch()` runs once on mount; subsequent refreshes are explicit. Every
 * getter returns `false` while loading or after a failed fetch so the UI
 * always fails closed on uncertain state.
 */
export function useCapability(): {
  sharingEnabled: ComputedRef<boolean>
  editableFolders: ComputedRef<boolean>
  shareGroups: ComputedRef<boolean>
  auditLog: ComputedRef<boolean>
  forkEnabled: ComputedRef<boolean>
  loading: ComputedRef<boolean>
  refresh: () => Promise<void>
} {
  const caps = capabilitiesStore()

  onMounted(() => {
    if (!caps.lastFetchedAt) {
      caps.fetch().catch(() => {
        // The store has already set fail-closed defaults; surfacing the
        // error in the UI is the caller's job (e.g. an admin banner).
      })
    }
  })

  return {
    sharingEnabled: computed(() => caps.sharingEnabled),
    editableFolders: computed(() => caps.editableFolders),
    shareGroups: computed(() => caps.shareGroups),
    auditLog: computed(() => caps.auditLog),
    forkEnabled: computed(() => caps.forkEnabled),
    loading: computed(() => caps.loading),
    refresh: () => caps.fetch()
  }
}
