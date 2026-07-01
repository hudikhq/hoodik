import { watch } from 'vue'
import { capabilitiesStore, trustedFingerprintsStore, store as sharesStore } from '!/shares'
import { store as loginStore } from '!/auth/login'

/**
 * Post-auth wiring for the shares subsystem. Mounted from
 * `LayoutAuthenticated` / `LayoutAuthenticatedClear` so the shares chunk
 * never enters the boot bundle that pre-auth routes (`/auth/login`,
 * `/auth/register`, …) ship. Pre-auth bundles include only `<RouterView />`
 * + the matched route's lazy chunk.
 *
 * Three jobs:
 *   1. fetch capabilities once per authenticated session
 *   2. rebind the local trusted-fingerprint map to the current user id
 *   3. wipe the shares store on logout
 */
export function useAuthedShareBootstrap(): void {
  const caps = capabilitiesStore()
  const login = loginStore()
  const trusted = trustedFingerprintsStore()
  const shares = sharesStore()

  watch(
    () => login.authenticated?.user.id ?? null,
    (userId, prev) => {
      trusted.bind(userId)
      if (userId && userId !== prev) {
        caps.fetch().catch(() => {
          // Fail-closed defaults are installed inside the store; swallow
          // the network error so unhandled rejections never bubble up to
          // the global notification listener.
        })
      }
      if (!userId) {
        shares.reset()
      }
    },
    { immediate: true }
  )
}
