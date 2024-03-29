export interface Data {
  users: Users
}

export interface Users {
  quota_bytes?: number
  allow_register: boolean
  enforce_email_activation: boolean
  email_whitelist: WhitelistOrBlacklist
  email_blacklist: WhitelistOrBlacklist
}

export interface WhitelistOrBlacklist {
  rules: string[]
}
