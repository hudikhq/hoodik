import type { Query } from '!/api'

export interface Search extends Query {
  with_expired?: boolean
  search?: string
  limit?: number
  offset?: number
  sort?: 'id' | 'email' | 'created_at' | 'expires_at'
  order?: 'asc' | 'desc'
}

export interface Create {
  email: string
  message?: string
  expires_at?: string
}

export interface Invitation {
  id: string
  user_id?: string
  email: string
  created_at: string
  expires_at: string
}
