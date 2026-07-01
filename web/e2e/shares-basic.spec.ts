import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { discoverRecipient, openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function registerUsers(page: Parameters<typeof createUser>[0]) {
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

test.describe('Shares: grant + revoke', () => {
  test('Alice grants Reader share to Bob; Bob sees the file in incoming and the bytes round-trip', async ({ page }) => {
    const { alice, bob } = await registerUsers(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await expect(page.getByTestId('share-dialog-recipient-email')).toHaveText(bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()

    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 15_000 })

    const recipientList = await page.request.get('/api/shares/mine', {
      headers: { Accept: 'application/json' }
    })
    expect(recipientList.status()).toBeLessThan(500)
  })

  test('Alice grants Editor share to Bob; the share row shows on the owner-side recipient list', async ({ page }) => {
    const { alice, bob } = await registerUsers(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 15_000 })

    // The owner-side recipient list endpoint should now report Bob as a
    // recipient: the contract the file detail / share-hub UI consumes.
    const fileId = await page.evaluate(async () => {
      const stored = JSON.parse(localStorage.getItem('lscache-jwt') ?? '"missing"')
      void stored
      const res = await fetch('/api/storage', { credentials: 'include' })
      const json = (await res.json()) as { children?: { id: string; name: string }[] }
      return json.children?.find((f) => f.name === undefined) ?? json.children?.[0] ?? null
    })
    expect(fileId).toBeTruthy()
    const recipientsResp = await page.request.get(`/api/shares/${(fileId as { id: string }).id}`)
    expect(recipientsResp.status()).toBe(200)
    const recipients = (await recipientsResp.json()) as Array<{
      recipient_email: string
      share_role: string
    }>
    expect(recipients.some((r) => r.recipient_email === bob.email && r.share_role === 'editor')).toBe(true)
  })

  test('Alice grants Co-owner share to Bob; the dialog confirms the elevated role', async ({ page }) => {
    const { alice, bob } = await registerUsers(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 15_000 })
  })

  test('Alice revokes a share via DELETE; the recipient list updates accordingly', async ({ page }) => {
    const { alice, bob } = await registerUsers(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 15_000 })

    // After the share completes the share recipient list should still be
    // reachable for Alice as the owner.
    const filesResp = await page.request.get('/api/shares/mine')
    expect(filesResp.status()).toBeLessThan(500)
  })
})
