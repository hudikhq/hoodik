import Api from '!/api'
import type { Search, Invitation, Create, Paginated } from 'types/admin/invitations'

/**
 * Get paginated array of the invitations sent to the potential new users
 */
export async function index(search: Search): Promise<Paginated> {
  const response = await Api.get<Paginated>(`/api/admin/invitations`, search)

  if (!response.body) {
    throw new Error('Failed to get invitations')
  }

  return response.body
}

/**
 * Create and send an invitation to the potential new user
 */
export async function create(create: Create): Promise<Invitation> {
  const response = await Api.post<Create, Invitation>(`/api/admin/invitations`, undefined, create)

  if (!response.body) {
    throw new Error('Failed to create invitation')
  }

  return response.body
}

/**
 * Expire an invitation so it cannot be used when registering
 */
export async function expire(id: string): Promise<void> {
  await Api.delete<Invitation>(`/api/admin/invitations/${id}`)
}
