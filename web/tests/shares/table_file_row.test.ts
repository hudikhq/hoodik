import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'

import TableFileRow from '../../src/views/files/index-view/TableFileRow.vue'
import { SHARED_WITH_ME_DIR_ID } from '../../services/storage'
import { isPreviewable } from '../../services/preview'
import type { AppFile } from '../../types'

const routerPush = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: routerPush })
}))

const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'

const SIZES = {
  checkbox: 'pl-2 pt-3 w-10 shrink-0',
  name: 'flex-1 p-2 pt-3 min-w-0 flex',
  size: 'hidden p-2 pt-3 md:block w-24 shrink-0',
  type: 'hidden p-2 pt-3 xl:block w-24 shrink-0',
  modifiedAt: 'hidden p-2 pt-3 sm:block w-44 shrink-0',
  buttons: 'w-10 p-2 shrink-0'
}

function baseFile(overrides: Partial<AppFile> = {}): AppFile {
  return {
    id: '11111111-1111-1111-1111-111111111111',
    user_id: OWNER_ID,
    is_owner: true,
    name: 'doc.pdf',
    name_hash: '',
    mime: 'application/pdf',
    chunks: 0,
    file_id: null,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: 'aegis128l',
    ...overrides
  } as unknown as AppFile
}

function mountRow(file: AppFile): ReturnType<typeof mount> {
  return mount(TableFileRow, {
    props: { file, checkedIds: new Set<string>(), sizes: SIZES }
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('TableFileRow: shared-out hint', () => {
  it('renders the share badge when the owner has shared the row out', () => {
    const wrapper = mountRow(baseFile({ shared_with_count: 2 }))
    const badge = wrapper.find('[data-testid="shared-out-badge"]')
    expect(badge.exists()).toBe(true)
    expect(badge.attributes('title')).toContain('2')
  })

  it('uses the singular phrasing for one recipient', () => {
    const wrapper = mountRow(baseFile({ shared_with_count: 1 }))
    const badge = wrapper.find('[data-testid="shared-out-badge"]')
    expect(badge.attributes('title')).toContain('1 other account')
  })

  it('omits the badge when the owner has shared with nobody', () => {
    const wrapper = mountRow(baseFile({ shared_with_count: 0 }))
    expect(wrapper.find('[data-testid="shared-out-badge"]').exists()).toBe(false)
  })

  it('omits the badge on non-owner rows even when the count is set', () => {
    // Incoming-share rows surface an "owned by" badge — the owner-side
    // share count would be misleading on them.
    const wrapper = mountRow(
      baseFile({ is_owner: false, shared_with_count: 5, share_role: 'reader' })
    )
    expect(wrapper.find('[data-testid="shared-out-badge"]').exists()).toBe(false)
  })
})

describe('TableFileRow: synthetic shared-with-me row', () => {
  it('renders a disabled checkbox on the synthetic root', () => {
    const wrapper = mountRow(
      baseFile({ id: SHARED_WITH_ME_DIR_ID, mime: 'dir', name: 'Shared with me' })
    )
    const checkbox = wrapper.find('input[type="checkbox"]')
    expect(checkbox.exists()).toBe(true)
    expect((checkbox.element as HTMLInputElement).disabled).toBe(true)
  })

  it('clicking the row name does not emit select-one on the synthetic row', async () => {
    const wrapper = mountRow(
      baseFile({ id: SHARED_WITH_ME_DIR_ID, mime: 'dir', name: 'Shared with me' })
    )
    // First click in the row's single-click handler would normally toggle
    // selection. The synthetic id short-circuits before the emit.
    await wrapper.find('button').trigger('click')
    expect(wrapper.emitted('select-one')).toBeFalsy()
  })

  it('regular rows still render an enabled checkbox', () => {
    const wrapper = mountRow(baseFile())
    const checkbox = wrapper.find('input[type="checkbox"]')
    expect(checkbox.exists()).toBe(true)
    expect((checkbox.element as HTMLInputElement).disabled).toBe(false)
  })
})

describe('isPreviewable: shared image without decrypted thumbnail', () => {
  // A shared image row arrives without a decrypted
  // thumbnail (the encrypted_thumbnail isn't always shipped to
  // recipients), but the file-preview view can render directly from
  // chunks. The previewable check must not block routing on thumbnail
  // presence for image and video mime types.
  it('accepts a shared image even when thumbnail is undefined', () => {
    const file = baseFile({
      is_owner: false,
      mime: 'image/png',
      size: 12_345,
      thumbnail: undefined,
      share_role: 'reader'
    })
    expect(isPreviewable(file)).toBe(true)
  })

  it('accepts a shared video even when thumbnail is undefined', () => {
    const file = baseFile({
      is_owner: false,
      mime: 'video/mp4',
      size: 999_999,
      thumbnail: undefined,
      share_role: 'reader'
    })
    expect(isPreviewable(file)).toBe(true)
  })

  it('rejects an unknown mime type when no thumbnail is present', () => {
    const file = baseFile({
      is_owner: false,
      mime: 'application/octet-stream',
      size: 100,
      thumbnail: undefined,
      share_role: 'reader'
    })
    expect(isPreviewable(file)).toBe(false)
  })
})

describe('TableFileRow: double-click routing on shared previewable rows', () => {
  beforeEach(() => {
    routerPush.mockClear()
  })

  function doubleClickRow(wrapper: ReturnType<typeof mountRow>): Promise<void> {
    // The component's single/double-click dispatcher requires two
    // discrete click events within its 250ms window; firing them in
    // sequence executes the double-click branch.
    const button = wrapper.find('button')
    button.trigger('click')
    return button.trigger('click') as Promise<void>
  }

  it('shared image with finished upload routes to file-preview (B3)', async () => {
    const wrapper = mountRow(
      baseFile({
        id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
        is_owner: false,
        mime: 'image/png',
        size: 12_345,
        finished_upload_at: 1_700_000_000,
        share_role: 'reader',
        thumbnail: undefined
      })
    )
    await doubleClickRow(wrapper)
    expect(routerPush).toHaveBeenCalledWith({
      name: 'file-preview',
      params: { id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee' }
    })
  })

  it('owned image still routes to file-preview', async () => {
    const wrapper = mountRow(
      baseFile({
        id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
        is_owner: true,
        mime: 'image/png',
        size: 12_345,
        finished_upload_at: 1_700_000_000,
        thumbnail: 'data:image/png;base64,XXX'
      })
    )
    await doubleClickRow(wrapper)
    expect(routerPush).toHaveBeenCalledWith({
      name: 'file-preview',
      params: { id: 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee' }
    })
  })
})
