import Api from '!/api'
import type { Paginated, Response, Search } from 'types/admin/users'

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
 * Disable users two factor
 */
export async function disableTfa(id: string): Promise<Response> {
  await Api.post<undefined, undefined>(`/api/admin/users/${id}/remove-tfa`)

  return get(id)
}

/**
 * Delete the user and all of its files
 */
export async function remove(id: string): Promise<void> {
  await Api.delete<undefined>(`/api/admin/users/${id}`)
}

/**
 * Get paginated array of users currently registered in the app
 */
export async function index(search: Search): Promise<Paginated> {
  const response = await Api.get<Paginated>(`/api/admin/users`, search)

  if (!response.body) {
    throw new Error('Failed to get users')
  }

  return response.body
}
