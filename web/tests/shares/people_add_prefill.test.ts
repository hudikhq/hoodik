import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'

import SharingPeopleAdd from '../../src/components/shares/SharingPeopleAdd.vue'
import { store as loginStore } from '../../services/auth/login'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'

import type { AppFile, KeyPair, ShareRole } from '../../types'

const OWNER_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
const RECIPIENT_ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'
const RECIPIENT_PUBKEY =
  '-----BEGIN RSA PUBLIC KEY-----\nMIIBCgKCAQEAtw1inWcpS34f9X0vsXB2e3rtx/80bcM7YAQhi3R+URcZaqFqjfQV\nOFEgTAaS+0rK1TUq0SPksTnCcfhKl5OAGMVQDOh0fTUV4+acvriiK0NGtgpne1hd\nDt04gTE0qKThoE2LuzlAVmh4S2mc+szi+PMsSojmuGqsklDh7yacsFRhMUKbWxax\n5hKJ6JzI0Ao9aYkq4E7YWrJVuANGlunvoBvxZKS9bg8+xCHP9mrXtLXdh4iS9qsf\n6T8AsPGTxO+pK9iwcWhciB57nQXTH7NgotJt+gcwJFE05UYjGCctjtkFjPyW2K0N\nMcxEGCf0KBCdRVBJAzL0xNylFmaSGRR56QIDAQAB\n-----END RSA PUBLIC KEY-----\n'

const FILE: AppFile = {
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
  cipher: 'aegis128l'
} as unknown as AppFile

function setupAuth(): void {
  const login = loginStore()
  login.set({
    user: { id: OWNER_ID, email: 'alice@example.com', pubkey: '', fingerprint: '' },
    session: {}
  } as unknown as Parameters<typeof login.set>[0])
}

function mountDialog(
  prefillEmail?: string,
  file: AppFile = FILE,
  prefillRole?: ShareRole,
  prefillAddFiles?: boolean
): ReturnType<typeof mount> {
  return mount(SharingPeopleAdd, {
    props: {
      file,
      authenticatedUserId: OWNER_ID,
      keypair: { input: 'priv', publicKey: 'pub' } as unknown as KeyPair,
      prefillEmail,
      prefillRole,
      prefillAddFiles
    } as unknown as Record<string, unknown>
  })
}

const FOLDER: AppFile = {
  ...FILE,
  id: '22222222-2222-2222-2222-222222222222',
  mime: 'dir',
  name: 'shared-folder'
} as unknown as AppFile

beforeEach(() => {
  setActivePinia(createPinia())
  if (typeof localStorage !== 'undefined') localStorage.clear()
  setupAuth()
  vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({ owned: [], member_of: [] })
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('SharingPeopleAdd: auto-find on prefill', () => {
  it('auto_find_fires_when_prefill_transitions_from_undefined_to_email', async () => {
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    const discoverSpy = vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: recipientFingerprint
    })

    const wrapper = mountDialog()
    await flushPromises()
    expect(discoverSpy).not.toHaveBeenCalled()

    await wrapper.setProps({ prefillEmail: 'bob@example.com' })
    await flushPromises()

    expect(discoverSpy).toHaveBeenCalledTimes(1)
    expect(discoverSpy).toHaveBeenCalledWith('bob@example.com')
    // The recipient card mounts once discover resolves — the role
    // picker becomes interactive without a Find user click.
    expect(wrapper.find('[data-testid="share-dialog-recipient-email"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="share-dialog-role-reader"]').exists()).toBe(true)
  })

  it('auto_find_skips_empty_and_whitespace_prefill', async () => {
    const discoverSpy = vi.spyOn(sharesApi, 'discoverUser')

    const wrapper = mountDialog()
    await flushPromises()
    await wrapper.setProps({ prefillEmail: '' })
    await flushPromises()
    await wrapper.setProps({ prefillEmail: '   ' })
    await flushPromises()

    expect(discoverSpy).not.toHaveBeenCalled()
  })

  it('auto_find_skips_when_phase_is_not_idle', async () => {
    // A second prefill landing mid-discover would spawn a second
    // sharesApi.discoverUser call before the first one resolves; the
    // guard pins the auto-fire to phase=idle so an in-flight lookup
    // wins the race.
    let resolveFirst: (value: unknown) => void = () => undefined
    const firstCall = new Promise((resolve) => {
      resolveFirst = resolve
    })
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    const discoverSpy = vi
      .spyOn(sharesApi, 'discoverUser')
      .mockImplementationOnce(async () => {
        await firstCall
        return {
          user_id: RECIPIENT_ID,
          email: 'bob@example.com',
          pubkey: RECIPIENT_PUBKEY,
          fingerprint: recipientFingerprint
        }
      })
      .mockResolvedValue({
        user_id: RECIPIENT_ID,
        email: 'carol@example.com',
        pubkey: RECIPIENT_PUBKEY,
        fingerprint: recipientFingerprint
      })

    const wrapper = mountDialog()
    await flushPromises()
    await wrapper.setProps({ prefillEmail: 'bob@example.com' })
    // Don't flush — the first discover stays in flight at phase=discovering.
    await wrapper.setProps({ prefillEmail: 'carol@example.com' })
    await flushPromises()
    // Only the first auto-fire registered while phase was idle.
    expect(discoverSpy).toHaveBeenCalledTimes(1)
    expect(discoverSpy).toHaveBeenCalledWith('bob@example.com')
    resolveFirst({})
    await flushPromises()
  })

  it('prefill_role_pre_selects_the_radio', async () => {
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: recipientFingerprint
    })

    const wrapper = mountDialog()
    await flushPromises()
    await wrapper.setProps({
      prefillEmail: 'bob@example.com',
      prefillRole: 'editor'
    })
    await flushPromises()

    const editorRadio = wrapper.find(
      '[data-testid="share-dialog-role-editor"]'
    ).element as HTMLInputElement
    expect(editorRadio.checked).toBe(true)
  })

  it('prefill_add_files_pre_checks_folder_toggle', async () => {
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: recipientFingerprint
    })

    const wrapper = mountDialog(undefined, FOLDER)
    await flushPromises()
    await wrapper.setProps({
      prefillEmail: 'bob@example.com',
      prefillRole: 'editor',
      prefillAddFiles: true
    })
    await flushPromises()

    const toggle = wrapper.find(
      '[data-testid="share-dialog-folder-editable-toggle"]'
    ).element as HTMLInputElement
    expect(toggle.checked).toBe(true)
  })

  it('reader_role_auto_unchecks_add_files_toggle', async () => {
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: recipientFingerprint
    })

    const wrapper = mountDialog(undefined, FOLDER)
    await flushPromises()
    await wrapper.setProps({
      prefillEmail: 'bob@example.com',
      prefillRole: 'editor',
      prefillAddFiles: true
    })
    await flushPromises()
    let toggle = wrapper.find(
      '[data-testid="share-dialog-folder-editable-toggle"]'
    ).element as HTMLInputElement
    expect(toggle.checked).toBe(true)

    // Flip the role to Reader; the watcher unchecks the add-files toggle
    // so a Reader recipient never lands on a disabled-but-checked state.
    await wrapper.find('[data-testid="share-dialog-role-reader"]').setValue()
    await flushPromises()
    toggle = wrapper.find(
      '[data-testid="share-dialog-folder-editable-toggle"]'
    ).element as HTMLInputElement
    expect(toggle.checked).toBe(false)
  })
})

describe('SharingPeopleAdd: submit overlay', () => {
  it('renders_overlay_with_status_during_submit', async () => {
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: recipientFingerprint
    })

    const wrapper = mountDialog()
    await flushPromises()
    await wrapper.setProps({ prefillEmail: 'bob@example.com' })
    await flushPromises()

    expect(wrapper.find('[data-testid="share-dialog-submit-overlay"]').exists()).toBe(false)

    // Pin the submit branch by toggling the exposed flag. The full submit
    // path needs subtree-collect + wasm signing that the jsdom harness
    // can't load; we cover that in the e2e suite instead. Vue unwraps
    // exposed refs on the proxy, so we assign the boolean directly.
    const exposed = wrapper.vm as unknown as { submitting: boolean }
    exposed.submitting = true
    await flushPromises()

    const overlay = wrapper.find('[data-testid="share-dialog-submit-overlay"]')
    expect(overlay.exists()).toBe(true)
    const status = wrapper.find('[data-testid="share-dialog-submit-overlay-status"]')
    expect(status.text()).toContain('bob@example.com')

    const cancelBtn = wrapper.findAll('button').find((b) => b.text() === 'Cancel')
    expect((cancelBtn?.element as HTMLButtonElement).disabled).toBe(true)
  })
})
