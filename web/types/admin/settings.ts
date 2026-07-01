export interface Data {
  users: Users
  sharing: Sharing
}

export interface Users {
  quota_bytes?: number
  allow_register: boolean
  enforce_email_activation: boolean
  email_whitelist: WhitelistOrBlacklist
  email_blacklist: WhitelistOrBlacklist
}

/**
 * Admin kill switch for account-to-account sharing.
 * Flipping to `false` makes every `/api/shares/...` endpoint return
 * `503 sharing_disabled` and tells `/api/capabilities` to advertise
 * `sharing.enabled = false` so clients hide the UI fail-closed.
 * Existing share rows are preserved across toggles.
 */
export interface Sharing {
  enabled: boolean
}

export interface WhitelistOrBlacklist {
  rules: string[]
}
