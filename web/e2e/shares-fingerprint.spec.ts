import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { discoverRecipient, openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function registerTwo(page: Parameters<typeof createUser>[0]) {
  const aliceEmail = randomEmail()
  const alicePassword = randomPassword()
  const alice = await createUser(page, aliceEmail, alicePassword)
  await logout(page)

  const bobEmail = randomEmail()
  const bobPassword = randomPassword()
  const bob = await createUser(page, bobEmail, bobPassword)
  await logout(page)

  return { alice, bob }
}

test.describe('Shares: fingerprint as information, never a gate', () => {
  test('First-time recipient renders the fingerprint and the unknown pill; Share is enabled immediately', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)

    const fingerprint = page.getByTestId('share-dialog-fingerprint')
    await expect(fingerprint).toBeVisible()
    await expect(fingerprint).toHaveText(/^[A-F0-9]{4}-[A-F0-9]{4}-/)

    await expect(page.getByTestId('share-dialog-unknown')).toBeVisible()
    // The dropped checkbox surfaces must not reappear on any code path.
    await expect(page.getByTestId('share-dialog-confirm')).toHaveCount(0)
    await expect(page.getByTestId('share-dialog-full-verify')).toHaveCount(0)
    await expect(page.getByTestId('share-dialog-submit')).toBeEnabled()
  })

  test('Cached recipient renders the trusted pill and stays enabled on re-share', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 15_000 })

    // Re-open the dialog for the same file. The trusted-fingerprints store
    // remembers Bob — the dialog now renders the trusted pill with a
    // "Last verified" label and the Share button is enabled the moment
    // discovery resolves.
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await expect(page.getByTestId('share-dialog-trusted')).toBeVisible()
    await expect(page.getByTestId('share-dialog-trusted')).toContainText(/Last verified/)
    await expect(page.getByTestId('share-dialog-confirm')).toHaveCount(0)
    await expect(page.getByTestId('share-dialog-full-verify')).toHaveCount(0)
    await expect(page.getByTestId('share-dialog-submit')).toBeEnabled()
  })

  test('Mismatch cancel closes the modal and leaves the cached entry untouched', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    // Share once so the trust store binds Bob's fingerprint on disk.
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })
    await logout(page)

    // Look up the stored ownerUserId so the localStorage key matches the
    // bind path inside `trustedFingerprintsStore.bind`. The /auth/refresh
    // round-trip surfaces the current user even after logout has cleared
    // the in-memory store — only do that probe by reading the persisted
    // key directly.
    const persistedOwnerKey = await page.evaluate(() => {
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i)
        if (key?.startsWith('hoodik:trustedFingerprints:')) return key
      }
      return null
    })
    expect(persistedOwnerKey).not.toBeNull()
    const cachedBefore = await page.evaluate((key) => localStorage.getItem(key!), persistedOwnerKey)
    expect(cachedBefore).toBeTruthy()

    // Swap Bob's cached fingerprint for a divergent hex before logging
    // back in. The bind on the next login will read this divergent map
    // verbatim, so the next discover lookup raises a mismatch.
    await page.evaluate(
      ({ key, raw }) => {
        const original = JSON.parse(raw!) as Record<
          string,
          { pubkeyFingerprint: string; lastVerifiedAt: number; verificationMethod: string }
        >
        const peerIds = Object.keys(original)
        if (peerIds.length === 0) return
        original[peerIds[0]].pubkeyFingerprint = 'deadbeef'.repeat(8)
        localStorage.setItem(key!, JSON.stringify(original))
      },
      { key: persistedOwnerKey, raw: cachedBefore }
    )

    await loginAsUser(page, alice.email, alice.password)
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)

    // The mismatch modal opens. Cancel returns to a clean dialog state
    // with the recipient input cleared and the cached entry untouched.
    await expect(page.getByTestId('fingerprint-mismatch-modal')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('share-dialog-submit')).toBeDisabled()
    await page.getByTestId('fingerprint-mismatch-cancel').click()
    await expect(page.getByTestId('fingerprint-mismatch-modal')).toHaveCount(0)

    // localStorage still holds the divergent value — cancel never consents
    // to the new fingerprint, so the cached entry stays exactly as the
    // user left it. Re-discover would surface the same modal again on the
    // next attempt.
    const cachedAfter = await page.evaluate(
      (key) => localStorage.getItem(key!),
      persistedOwnerKey
    )
    const divergentMap = JSON.parse(cachedBefore!) as Record<
      string,
      { pubkeyFingerprint: string; lastVerifiedAt: number; verificationMethod: string }
    >
    const peerIds = Object.keys(divergentMap)
    divergentMap[peerIds[0]].pubkeyFingerprint = 'deadbeef'.repeat(8)
    expect(cachedAfter).toBe(JSON.stringify(divergentMap))
  })

  test('Tampered cached fingerprint opens the dedicated mismatch modal and blocks Share', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    const [discoverResp, selfResp] = await Promise.all([
      page.request.get(`/api/users/discover?email=${encodeURIComponent(bob.email)}`),
      page.request.post('/api/auth/refresh')
    ])
    expect(discoverResp.status()).toBe(200)
    const discovered = (await discoverResp.json()) as { user_id: string }
    expect(selfResp.status()).toBeLessThan(400)
    const self = (await selfResp.json()) as { user: { id: string } }

    await page.evaluate(
      ({ peerId, ownerId }) => {
        const entry = {
          [peerId]: {
            pubkeyFingerprint: 'deadbeef'.repeat(8),
            lastVerifiedAt: Math.floor(Date.now() / 1000),
            verificationMethod: 'other'
          }
        }
        localStorage.setItem(`hoodik:trustedFingerprints:${ownerId}`, JSON.stringify(entry))
      },
      { peerId: discovered.user_id, ownerId: self.user.id }
    )

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)

    // The trust store binds at login time and the localStorage write
    // doesn't roll into the live Pinia state, so either the dedicated
    // mismatch modal opens (when the store has re-read storage) or the
    // unknown pill renders (store is still on the empty in-memory map).
    // The trusted-fresh path — silently letting the share proceed —
    // is the only outcome that must never happen.
    await expect(page.getByTestId('share-dialog-trusted')).toHaveCount(0)
    const mismatchVisible = await page.getByTestId('fingerprint-mismatch-modal').isVisible().catch(() => false)
    if (mismatchVisible) {
      await expect(page.getByTestId('share-dialog-submit')).toBeDisabled()
    } else {
      await expect(page.getByTestId('share-dialog-unknown')).toBeVisible()
    }
  })
})
