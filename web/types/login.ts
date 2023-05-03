export interface Authenticated {
  user: User
  session: Session
}

export interface AuthenticatedJwt {
  authenticated: Authenticated
  jwt: string
}

export interface User {
  id: string
  email: string
  private?: string
  pubkey: string
  fingerprint: string
  encrypted_private_key?: string
  created_at: string
  updated_at: string
}

export interface Session {
  id: string
  user_id: string
  token: string
  device_id?: string
  csrf: string
  created_at: string
  updated_at: string
  expires_at: string
}

export interface Credentials {
  email: string
  password: string
  token?: string
  remember?: boolean
  privateKey?: string
}

export interface PrivateKeyLogin {
  privateKey: string
  remember?: boolean
}
