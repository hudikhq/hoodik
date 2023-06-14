import type { User } from 'types'
import type { Stats } from './files'
import type { Query } from '!/api'

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
