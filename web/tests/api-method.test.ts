import { describe, it, expect } from 'vitest'
import Api from '../services/api'

describe('Api request method casing', () => {
  // fetch() uppercases GET, POST, PUT, DELETE, HEAD and OPTIONS on its own
  // but leaves PATCH untouched, and actix matches methods case-sensitively.
  // A lowercase 'patch' reached the server as `patch` and 404'd on a route
  // registered for PATCH.
  it('sends PATCH uppercased', () => {
    const { request, fetchOptions } = Api.buildRequest('patch', '/api/users/me', undefined, {})

    expect(fetchOptions.method).toBe('PATCH')
    expect(request.method).toBe('PATCH')
  })

  it.each(['get', 'post', 'put', 'delete'] as const)('sends %s uppercased', (method) => {
    const { fetchOptions } = Api.buildRequest(method, '/api/test')

    expect(fetchOptions.method).toBe(method.toUpperCase())
  })
})
