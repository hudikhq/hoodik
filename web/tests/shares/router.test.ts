import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as sharesApi from '../../services/shares/api'
import { capabilitiesStore } from '../../services/shares'

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetModules()
})

afterEach(() => {
  vi.restoreAllMocks()
})

async function freshRouter() {
  // Importing the router module after `vi.resetModules()` rebuilds the
  // singleton against the freshly-installed Pinia, so the capability
  // guard sees the right store on every test.
  const mod = await import('../../src/router/index')
  return mod.default
}

describe('Share router', () => {
  it('share_route_redirects_to_share_public_by_default', async () => {
    vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue({
      sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
      editable_folders: true,
      share_groups: true,
      audit_log: true,
      fork: true
    })
    await capabilitiesStore().fetch()
    const router = await freshRouter()
    await router.push('/share')
    await router.isReady()
    expect(router.currentRoute.value.name).toEqual('share-public')
  })

  it('links_route_redirects_to_share_public', async () => {
    const router = await freshRouter()
    await router.push('/links')
    await router.isReady()
    expect(router.currentRoute.value.fullPath).toEqual('/share/public')
  })

  it('links_subpath_preserves_in_redirect', async () => {
    const router = await freshRouter()
    await router.push('/links/edit/123')
    await router.isReady()
    expect(router.currentRoute.value.fullPath).toEqual('/share/public/edit/123')
  })

  it('with_me_route_redirects_to_virtual_folder_in_files', async () => {
    const router = await freshRouter()
    await router.push('/share/with-me')
    await router.isReady()
    expect(router.currentRoute.value.name).toEqual('files')
    expect(router.currentRoute.value.params.file_id).toEqual('__shared_with_me__')
  })

  it('mine_route_redirects_to_share_public', async () => {
    const router = await freshRouter()
    await router.push('/share/mine')
    await router.isReady()
    expect(router.currentRoute.value.name).toEqual('share-public')
  })

  it('audit_route_redirects_to_share_activity', async () => {
    vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue({
      sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
      editable_folders: true,
      share_groups: true,
      audit_log: true,
      fork: true
    })
    await capabilitiesStore().fetch()
    const router = await freshRouter()
    await router.push('/share/audit')
    await router.isReady()
    expect(router.currentRoute.value.name).toEqual('share-activity')
    expect(router.currentRoute.value.fullPath).toEqual('/share/activity')
  })

  it('share_activity_falls_back_to_public_when_audit_disabled', async () => {
    vi.spyOn(sharesApi, 'getCapabilities').mockResolvedValue({
      sharing: { enabled: true, roles: ['reader', 'editor', 'co-owner'] },
      editable_folders: true,
      share_groups: true,
      audit_log: false,
      fork: true
    })
    await capabilitiesStore().fetch()
    const router = await freshRouter()
    await router.push('/share/activity')
    await router.isReady()
    expect(router.currentRoute.value.name).toEqual('share-public')
  })

  it('sidebar_link_navigates_to_share', async () => {
    // The sidebar menu entry resolves through Vue Router via { name: 'share' };
    // confirm that route name exists and resolves to the parent layout.
    const router = await freshRouter()
    const resolved = router.resolve({ name: 'share' })
    expect(resolved.path).toEqual('/share')
  })
})
