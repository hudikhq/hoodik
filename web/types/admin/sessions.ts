import type { Query } from '!/api'

export interface Session {
  id: string
  user_id: string
  email: string
  ip: string
  user_agent: String
  created_at: string
  updated_at: string
  expires_at: string
  deleted_at?: string
}

export interface Search extends Query {
  with_deleted?: boolean
  with_expired?: boolean
  user_id?: string
  search?: string
  sort?: 'id' | 'email' | 'created_at' | 'updated_at' | 'expires_at'
  order?: 'asc' | 'desc'
  limit?: number
  offset?: number
}
