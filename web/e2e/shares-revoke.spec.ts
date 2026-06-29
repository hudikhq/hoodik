import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import {
  discoverRecipient,
  openShareDialogFor,
  openRowActions,
  openSharedWithMe
} from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function registerTwo(page: Parameters<typeof createUser>[0]) {
  const alice = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const bob = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  return { alice, bob }
}

test.describe('Shares: revoke', () => {
  test('Alice revokes Bob from the Sharing modal; recipient list reports empty', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    const filesList = (await (await page.request.get('/api/storage')).json()) as {
      children: { id: string; encrypted_name: string }[]
    }
    const file = filesList.children?.[0]
    expect(file).toBeTruthy()
    const beforeResp = await page.request.get(`/api/shares/${file.id}`)
    const beforeRows = (await beforeResp.json()) as { recipient_email: string }[]
    expect(beforeRows.some((r) => r.recipient_email === bob.email)).toBe(true)

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')

    // Drive the revoke through the unified Sharing modal: row → Sharing →
    // People tab → revoke button on Bob's row.
    await openRowActions(page, 'test-image.png')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('sharing-modal-tab-people')).toBeVisible({ timeout: 15_000 })
    const revokeRow = page.locator(`[data-testid^="sharing-modal-people-row-"]`).first()
    await expect(revokeRow).toBeVisible({ timeout: 15_000 })
    await revokeRow.locator('[data-testid^="sharing-modal-revoke-"]').click()
    // BaseButtonConfirm swaps into the confirm/cancel pair on first click.
    await revokeRow.locator('[data-testid^="sharing-modal-revoke-"]').locator('button').first().click()

    // The recipient list now reports empty.
    await expect(async () => {
      const afterResp = await page.request.get(`/api/shares/${file.id}`)
      const afterRows = (await afterResp.json()) as { recipient_email: string }[]
      expect(afterRows.some((r) => r.recipient_email === bob.email)).toBe(false)
    }).toPass({ timeout: 15_000 })
  })

  test('Bob self-removes from the virtual folder Leave action; Alice still has the file', async ({ page }) => {
    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    const filesList = (await (await page.request.get('/api/storage')).json()) as {
      children: { id: string }[]
    }
    const file = filesList.children?.[0]
    expect(file).toBeTruthy()

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    const sharedRow = page.getByTestId('file-row-test-image.png')
    await expect(sharedRow).toBeVisible({ timeout: 15_000 })
    await openRowActions(page, 'test-image.png')
    await page.locator('[data-testid="actions-leave"]').first().click()
    // Self-remove opens its own first-person confirmation modal.
    await expect(page.getByTestId('revoke-confirm-modal-self')).toBeVisible({ timeout: 5_000 })
    await page.getByTestId('revoke-confirm-modal-accept').click()
    await expect(sharedRow).toHaveCount(0, { timeout: 15_000 })

    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    const aliceFiles = (await (await page.request.get('/api/storage')).json()) as {
      children: { id: string }[]
    }
    expect(aliceFiles.children?.some((f) => f.id === file.id)).toBe(true)
  })
})
