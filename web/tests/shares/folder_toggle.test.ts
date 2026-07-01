import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'

import SharingPeopleAdd from '../../src/components/shares/SharingPeopleAdd.vue'
import { store as loginStore } from '../../services/auth/login'

import type { AppFile, KeyPair } from '../../types'

const FOLDER_FILE: AppFile = {
  id: '11111111-1111-1111-1111-111111111111',
  user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
  is_owner: true,
  name: 'shared-folder',
  name_hash: '',
  mime: 'dir',
  chunks: 0,
  file_id: null,
  file_modified_at: 0,
  created_at: 0,
  is_new: false,
  editable: false,
  active_version: 1,
  encrypted_key: '',
  encrypted_name: '',
  cipher: 'aegis128l'
} as unknown as AppFile

const FILE_FILE: AppFile = {
  ...FOLDER_FILE,
  id: '22222222-2222-2222-2222-222222222222',
  name: 'report.pdf',
  mime: 'application/pdf'
} as unknown as AppFile

function setupAuth(): void {
  const login = loginStore()
  login.set({
    user: {
      id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
      email: 'tester@example.com',
      pubkey: '',
      fingerprint: ''
    },
    session: {}
  } as unknown as Parameters<typeof login.set>[0])
}

function mountDialog(file: AppFile): ReturnType<typeof mount> {
  return mount(SharingPeopleAdd, {
    props: {
      file,
      authenticatedUserId: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
      keypair: { input: 'priv', publicKey: 'pub' } as unknown as KeyPair
    }
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
  setupAuth()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('share dialog folder-editable toggle', () => {
  it('share_dialog_shows_folder_toggle_only_for_directories', async () => {
    // Need a discovered recipient first — the toggle lives inside the
    // recipient panel because role selection requires a recipient.
    const wrapper = mountDialog(FOLDER_FILE)
    await flushPromises()
    // Without a discovered recipient the dialog renders the input but
    // not the role panel — verify the toggle is gated on the same
    // condition by NOT being present yet.
    expect(
      wrapper.find('[data-testid="share-dialog-folder-editable"]').exists()
    ).toBe(false)
    // After mocking a recipient lookup the panel renders.
    const sharesApi = await import('../../services/shares/api')
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      email: 'bob@example.com',
      pubkey:
        '-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEAtw1inWcpS34f9X0vsXB2e3rtx/80bcM7YAQhi3R+URcZaqFqjfQV\nOFEgTAaS+0rK1TUq0SPksTnCcfhKl5OAGMVQDOh0fTUV4+acvriiK0NGtgpne1hd\nDt04gTE0qKThoE2LuzlAVmh4S2mc+szi+PMsSojmuGqsklDh7yacsFRhMUKbWxax\n5hKJ6JzI0Ao9aYkq4E7YWrJVuANGlunvoBvxZKS9bg8+xCHP9mrXtLXdh4iS9qsf\n6T8AsPGTxO+pK9iwcWhciB57nQXTH7NgotJt+gcwJFE05UYjGCctjtkFjPyW2K0N\nMcxEGCf0KBCdRVBJAzL0xNylFmaSGRR56QIDAQAB\n-----END RSA PUBLIC KEY-----\n',
      fingerprint: 'a'.repeat(64)
    })
    await wrapper.get('input[name="recipient-email"]').setValue('bob@example.com')
    await wrapper.get('[data-testid="share-dialog-discover"]').trigger('click')
    await flushPromises()
    expect(
      wrapper.find('[data-testid="share-dialog-folder-editable"]').exists()
    ).toBe(true)

    // For a non-folder, the toggle is omitted entirely.
    const fileWrapper = mountDialog(FILE_FILE)
    await flushPromises()
    await fileWrapper.get('input[name="recipient-email"]').setValue('bob@example.com')
    await fileWrapper.get('[data-testid="share-dialog-discover"]').trigger('click')
    await flushPromises()
    expect(
      fileWrapper.find('[data-testid="share-dialog-folder-editable"]').exists()
    ).toBe(false)
  })

  it('share_dialog_folder_toggle_disabled_for_reader_role', async () => {
    const wrapper = mountDialog(FOLDER_FILE)
    const sharesApi = await import('../../services/shares/api')
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      email: 'bob@example.com',
      pubkey:
        '-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEAtw1inWcpS34f9X0vsXB2e3rtx/80bcM7YAQhi3R+URcZaqFqjfQV\nOFEgTAaS+0rK1TUq0SPksTnCcfhKl5OAGMVQDOh0fTUV4+acvriiK0NGtgpne1hd\nDt04gTE0qKThoE2LuzlAVmh4S2mc+szi+PMsSojmuGqsklDh7yacsFRhMUKbWxax\n5hKJ6JzI0Ao9aYkq4E7YWrJVuANGlunvoBvxZKS9bg8+xCHP9mrXtLXdh4iS9qsf\n6T8AsPGTxO+pK9iwcWhciB57nQXTH7NgotJt+gcwJFE05UYjGCctjtkFjPyW2K0N\nMcxEGCf0KBCdRVBJAzL0xNylFmaSGRR56QIDAQAB\n-----END RSA PUBLIC KEY-----\n',
      fingerprint: 'a'.repeat(64)
    })
    await wrapper.get('input[name="recipient-email"]').setValue('bob@example.com')
    await wrapper.get('[data-testid="share-dialog-discover"]').trigger('click')
    await flushPromises()
    // Reader is the default role; the toggle should be disabled and the
    // disabled-hint visible.
    const toggle = wrapper.get('[data-testid="share-dialog-folder-editable-toggle"]')
    expect((toggle.element as HTMLInputElement).disabled).toBe(true)
    expect(
      wrapper.find('[data-testid="share-dialog-folder-editable-disabled-hint"]').exists()
    ).toBe(true)
  })

  it('share_dialog_folder_toggle_enabled_for_editor_and_co_owner_roles', async () => {
    const wrapper = mountDialog(FOLDER_FILE)
    const sharesApi = await import('../../services/shares/api')
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
      email: 'bob@example.com',
      pubkey:
        '-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEAtw1inWcpS34f9X0vsXB2e3rtx/80bcM7YAQhi3R+URcZaqFqjfQV\nOFEgTAaS+0rK1TUq0SPksTnCcfhKl5OAGMVQDOh0fTUV4+acvriiK0NGtgpne1hd\nDt04gTE0qKThoE2LuzlAVmh4S2mc+szi+PMsSojmuGqsklDh7yacsFRhMUKbWxax\n5hKJ6JzI0Ao9aYkq4E7YWrJVuANGlunvoBvxZKS9bg8+xCHP9mrXtLXdh4iS9qsf\n6T8AsPGTxO+pK9iwcWhciB57nQXTH7NgotJt+gcwJFE05UYjGCctjtkFjPyW2K0N\nMcxEGCf0KBCdRVBJAzL0xNylFmaSGRR56QIDAQAB\n-----END RSA PUBLIC KEY-----\n',
      fingerprint: 'a'.repeat(64)
    })
    await wrapper.get('input[name="recipient-email"]').setValue('bob@example.com')
    await wrapper.get('[data-testid="share-dialog-discover"]').trigger('click')
    await flushPromises()
    // Promote to Editor.
    await wrapper.get('[data-testid="share-dialog-role-editor"]').setValue(true)
    await flushPromises()
    let toggle = wrapper.get('[data-testid="share-dialog-folder-editable-toggle"]')
    expect((toggle.element as HTMLInputElement).disabled).toBe(false)
    // Promote to Co-owner.
    await wrapper.get('[data-testid="share-dialog-role-coowner"]').setValue(true)
    await flushPromises()
    toggle = wrapper.get('[data-testid="share-dialog-folder-editable-toggle"]')
    expect((toggle.element as HTMLInputElement).disabled).toBe(false)
  })
})
