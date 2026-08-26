import { describe, it, expect } from 'vitest'
import Api from '../services/api'

/**
 * The server refuses writes from a client too old to produce the shapes it
 * stores, and reads this header to decide. A request that arrived without it
 * would be indistinguishable from one sent by a client from before the header
 * existed — which is the population the check exists to refuse.
 */
describe('the client identity header', () => {
  it.each(['get', 'post', 'put', 'delete'] as const)('goes out on %s', (method) => {
    const { request } = Api.buildRequest(method, '/api/test')

    expect(request.headers['X-Hoodik-Client']).toBe('web')
  })

  it('carries no version', () => {
    // This bundle is compiled into the binary that serves it, so it cannot be
    // the stale side. A version here would be a number to keep in step, and
    // one that drifted low would lock the app out of its own server.
    const { request } = Api.buildRequest('post', '/api/test')

    expect(request.headers['X-Hoodik-Client']).not.toMatch(/\d/)
  })

  it('survives caller-supplied headers', () => {
    const { request } = Api.buildRequest('post', '/api/test', undefined, {}, {
      'Content-Type': 'text/plain'
    })

    expect(request.headers['X-Hoodik-Client']).toBe('web')
    expect(request.headers['Content-Type']).toBe('text/plain')
  })
})
