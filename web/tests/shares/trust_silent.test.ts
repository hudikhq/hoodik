import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'

import SharingPeopleAdd from '../../src/components/shares/SharingPeopleAdd.vue'
import { trustedFingerprintsStore } from '../../services/shares'
import { store as loginStore } from '../../services/auth/login'
import * as sharesApi from '../../services/shares/api'
import * as shareCrypto from '../../services/shares/crypto'

import type { AppFile, KeyPair } from '../../types'

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

function bindTrust(): ReturnType<typeof trustedFingerprintsStore> {
  const trusted = trustedFingerprintsStore()
  trusted.bind(OWNER_ID)
  return trusted
}

function mountDialog(): ReturnType<typeof mount> {
  return mount(SharingPeopleAdd, {
    props: {
      file: FILE,
      authenticatedUserId: OWNER_ID,
      keypair: { input: 'priv', publicKey: 'pub' } as unknown as KeyPair
    }
  })
}

async function discover(wrapper: ReturnType<typeof mount>): Promise<void> {
  await wrapper.get('input[name="recipient-email"]').setValue('bob@example.com')
  await wrapper.get('[data-testid="share-dialog-discover"]').trigger('click')
  await flushPromises()
}

beforeEach(() => {
  setActivePinia(createPinia())
  if (typeof localStorage !== 'undefined') {
    localStorage.clear()
  }
  setupAuth()
  // The dialog calls listGroups on mount; mock it to a flat empty
  // response so the discover path stays in single-recipient mode.
  vi.spyOn(sharesApi, 'listGroups').mockResolvedValue({ owned: [], member_of: [] })
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('share dialog: fingerprint as information, not a gate', () => {
  it('enables Share immediately for a trusted-fresh recipient', async () => {
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: recipientFingerprint
    })

    const trusted = bindTrust()
    trusted.trustFingerprint(RECIPIENT_ID, recipientFingerprint, 'in-person')

    const wrapper = mountDialog()
    await flushPromises()
    await discover(wrapper)

    expect(wrapper.find('[data-testid="share-dialog-trusted"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="share-dialog-unknown"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="share-dialog-confirm"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="share-dialog-full-verify"]').exists()).toBe(false)
    expect(
      (wrapper.find('[data-testid="share-dialog-submit"]').element as HTMLButtonElement)
        .disabled
    ).toBe(false)
  })

  it('enables Share immediately on first contact and renders the unknown pill', async () => {
    const recipientFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: recipientFingerprint
    })

    bindTrust()
    const wrapper = mountDialog()
    await flushPromises()
    await discover(wrapper)

    expect(wrapper.find('[data-testid="share-dialog-unknown"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="share-dialog-confirm"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="share-dialog-full-verify"]').exists()).toBe(false)
    expect(
      (wrapper.find('[data-testid="share-dialog-submit"]').element as HTMLButtonElement)
        .disabled
    ).toBe(false)
  })

  it('blocks Share via the mismatch modal when the cached fingerprint disagrees', async () => {
    const trueFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    const cachedFingerprint = 'deadbeef'.repeat(8)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: trueFingerprint
    })

    const trusted = bindTrust()
    trusted.trustFingerprint(RECIPIENT_ID, cachedFingerprint, 'in-person')

    const wrapper = mountDialog()
    await flushPromises()
    await discover(wrapper)

    expect(wrapper.find('[data-testid="fingerprint-mismatch-modal"]').exists()).toBe(true)
    expect(
      (wrapper.find('[data-testid="share-dialog-submit"]').element as HTMLButtonElement)
        .disabled
    ).toBe(true)
  })

  it('records the new trust entry and enables Share when the user accepts the mismatch', async () => {
    const trueFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    const cachedFingerprint = 'deadbeef'.repeat(8)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: trueFingerprint
    })

    const trusted = bindTrust()
    trusted.trustFingerprint(RECIPIENT_ID, cachedFingerprint, 'in-person')

    const wrapper = mountDialog()
    await flushPromises()
    await discover(wrapper)

    await wrapper.get('[data-testid="fingerprint-mismatch-accept"]').trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="fingerprint-mismatch-modal"]').exists()).toBe(false)
    expect(trusted.lookup(RECIPIENT_ID)?.pubkeyFingerprint).toBe(trueFingerprint)
    expect(
      (wrapper.find('[data-testid="share-dialog-submit"]').element as HTMLButtonElement)
        .disabled
    ).toBe(false)
  })

  it('clears the recipient when the user cancels the mismatch and leaves the cached entry alone', async () => {
    const trueFingerprint = shareCrypto.computeFingerprint(RECIPIENT_PUBKEY)
    const cachedFingerprint = 'deadbeef'.repeat(8)
    vi.spyOn(sharesApi, 'discoverUser').mockResolvedValue({
      user_id: RECIPIENT_ID,
      email: 'bob@example.com',
      pubkey: RECIPIENT_PUBKEY,
      fingerprint: trueFingerprint
    })

    const trusted = bindTrust()
    trusted.trustFingerprint(RECIPIENT_ID, cachedFingerprint, 'in-person')

    const wrapper = mountDialog()
    await flushPromises()
    await discover(wrapper)

    await wrapper.get('[data-testid="fingerprint-mismatch-cancel"]').trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="fingerprint-mismatch-modal"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="share-dialog-recipient-email"]').exists()).toBe(false)
    expect(trusted.lookup(RECIPIENT_ID)?.pubkeyFingerprint).toBe(cachedFingerprint)
  })
})
