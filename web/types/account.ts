import type { Query } from '!/api'

export interface UnsecureChangePassword extends ChangePassword {
  use_private_key: boolean
  private_key: string
}

export interface ChangePassword {
  current_password?: string
  signature?: string
  token?: string
  email: string
  encrypted_private_key: string
  password: string
}

export interface ActivityQuery extends Query {
  with_expired?: boolean
  search?: string
  sort?: 'id' | 'created_at' | 'updated_at' | 'expires_at'
  order?: 'asc' | 'desc'
  limit?: number
  offset?: number
}
