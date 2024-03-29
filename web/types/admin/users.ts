import type { Stats } from './files'
import type { Query } from '!/api'
import type { Session } from './sessions'

export interface Update {
  role?: string
  quota?: number
}

export interface User {
  id: string
  role?: string
  email: string
  secret: boolean
  quota?: number
  pubkey: string
  fingerprint: string
  email_verified_at?: number
  created_at: number
  updated_at: number
  last_session?: Session
}

export interface Response {
  user: User
  stats: Stats[]
}

export interface Search extends Query {
  search?: string
  sort?: 'id' | 'email' | 'created_at' | 'updated_at'
  order?: 'asc' | 'desc'
  limit?: number
  offset?: number
}
