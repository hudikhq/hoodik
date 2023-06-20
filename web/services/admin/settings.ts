import Api from '!/api'
import type { Data } from 'types/admin/settings'

/**
 * Update application settings
 */
export async function update(data: Data): Promise<Data> {
  const response = await Api.put<Data, Data>(`/api/admin/settings`, undefined, data)

  if (!response.body) {
    throw new Error('Failed to update settings')
  }

  return response.body
}

/**
 * Get paginated array of settings currently registered in the app
 */
export async function index(): Promise<Data> {
  const response = await Api.get<undefined>(`/api/admin/settings`)

  if (!response.body) {
    throw new Error('Failed to get settings')
  }

  return response.body
}
