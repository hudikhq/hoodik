import Api from '!/api'
import type { Paginated } from 'types'
import type { Search, Session } from 'types/admin/sessions'

/**
 * Get paginated array of the sessions sent to the potential new users
 */
export async function index(search: Search): Promise<Paginated<Session>> {
  const response = await Api.get<Paginated<Session>>(`/api/admin/sessions`, search)

  if (!response.body) {
    throw new Error('Failed to get sessions')
  }

  return response.body
}

/**
 * Kill session by its id
 */
export async function kill(id: string): Promise<void> {
  await Api.delete(`/api/admin/sessions/${id}`)
}

/**
 * Kill all the active sessions for the user
 */
export async function killForUser(user_id: string): Promise<void> {
  await Api.delete(`/api/admin/sessions/${user_id}/kill-for-user`)
}
