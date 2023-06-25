import Api from '!/api'
import type { Response } from 'types/admin/files'

/**
 * Get general stats of file system, available storage and file types
 */
export async function stats(): Promise<Response> {
  const response = await Api.get<Response>(`/api/admin/files`)

  if (!response.body) {
    throw new Error('Failed to get file stats')
  }

  return response.body
}
