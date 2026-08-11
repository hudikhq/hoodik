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
  /** Set when the advertisement could not be fetched, as opposed to the
   *  operator genuinely having sharing switched off. */
  fetchError: ComputedRef<string | null>
  refresh: () => Promise<void>
} {
  const caps = capabilitiesStore()

  onMounted(() => {
    if (!caps.lastFetchedAt) {
      caps.fetch().catch(() => {
        // Fail-closed defaults are already installed by the store, and that
        // is the right security posture. `fetchError` is what lets the view
        // say the features are missing because of a network failure rather
        // than an operator decision.
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
    fetchError: computed(() => caps.fetchError),
    refresh: () => caps.fetch()
  }
}
