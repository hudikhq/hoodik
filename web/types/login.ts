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
  role?: string
  created_at: number
  updated_at: number
  email_verified_at?: number
  secret: boolean
}

export interface Session {
  id: string
  user_id: string
  device_id?: string
  ip: string
  refresh: boolean
  user_agent: string
  created_at: number
  updated_at: number
  expires_at: number
}

export interface Credentials {
  email: string
  password: string
  token?: string
  privateKey?: string
  remember?: boolean
}

export interface PrivateKeyLogin {
  privateKey: string
  remember?: boolean
}
