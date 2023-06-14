import Api from '!/api'
import type { User } from 'types'
import type { Response, Search } from 'types/admin/users'

/**
 * Get single usee, its object and stats
 */
export async function get(id: string): Promise<Response> {
  const response = await Api.get<Response>(`/api/admin/users/${id}`)

  if (!response.body) {
    throw new Error('Failed to get user')
  }

  return response.body
}

/**
 * Get paginated array of users currently registered in the app
 */
export async function index(search: Search): Promise<User[]> {
  const response = await Api.get<User[]>(`/api/admin/users`, search)

  if (!response.body) {
    throw new Error('Failed to get users')
  }

  return response.body
}
