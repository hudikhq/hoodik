import type { Query } from '!/api'

export interface Paginated {
  sessions: Session[]
  total: number
}

export interface Session {
  id: string
  user_id: string
  email: string
  ip: string
  user_agent: String
  created_at: number
  updated_at: number
  expires_at: number
  active: boolean
}

export interface Search extends Query {
  with_expired?: boolean
  user_id?: string
  search?: string
  sort?: 'id' | 'email' | 'created_at' | 'updated_at' | 'expires_at'
  order?: 'asc' | 'desc'
  limit?: number
  offset?: number
}
