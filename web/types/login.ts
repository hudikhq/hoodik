export interface Authenticated {
  user: User
  session: Session
}

export interface User {
  id: string
  email: string
  pubkey: string
  fingerprint: string
  encrypted_private_key?: string
  created_at: string
  updated_at: string
  email_verified_at?: string
}

export interface Session {
  id: string
  user_id: string
  device_id?: string
  ip: string
  user_agent: string
  created_at: string
  updated_at: string
  expires_at: string
}

export interface Credentials {
  email: string
  password: string
  token?: string
  privateKey?: string
}

export interface PrivateKeyLogin {
  privateKey: string
  remember?: boolean
}
