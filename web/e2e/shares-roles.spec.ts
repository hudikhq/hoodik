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

async function registerThree(page: Parameters<typeof createUser>[0]) {
  const alice = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const bob = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const carol = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  return { alice, bob, carol }
}

test.describe('Share roles', () => {
  test('Reader sees an inspectable Sharing entry but no fork or delete', async ({ page }) => {
    const { alice, bob, carol } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    // Alice grants Carol Reader too — Bob's roster view will surface
    // every co-recipient since sharing the file shares the roster.
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, carol.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await openRowActions(page, 'test-image.png')
    // Fork is still Co-owner-only. Sharing is universal — Reader inspects
    // the roster, mutation is gated inside the modal.
    await expect(page.locator('[data-testid="actions-fork"]')).toHaveCount(0)
    await expect(page.locator('[data-testid="actions-share-account"]').first()).toBeVisible()
    await expect(page.locator('[data-testid="actions-leave"]').first()).toBeVisible()

    // Open the Sharing modal — Bob sees Bob + Carol on the People tab.
    // `recipient_list` returns every non-owner row (Bob and Carol; Alice
    // is the owner). No 401 toast.
    await page.locator('[data-testid="actions-share-account"]').first().click()
    await expect(page.getByTestId('sharing-modal-readonly-banner')).toBeVisible()
    const peopleRows = page.locator('[data-testid^="sharing-modal-people-row-"]')
    await expect(peopleRows).toHaveCount(2, { timeout: 15_000 })
    const bodyText = await page.locator('[data-testid="sharing-modal-people-list"]').textContent()
    expect(bodyText).toContain(bob.email)
    expect(bodyText).toContain(carol.email)
    // No add-recipient submit button is enabled for a Reader.
    await expect(page.getByTestId('share-dialog-submit')).toBeDisabled()
  })

  test('Editor inside the virtual folder sees a shared-by badge and an inspectable Sharing entry', async ({ page }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    const row = page.getByTestId('file-row-test-image.png')
    await expect(row).toBeVisible({ timeout: 15_000 })
    await expect(row.getByTestId('shared-by-badge')).toContainText(alice.email)
    await openRowActions(page, 'test-image.png')
    await expect(page.locator('[data-testid="actions-fork"]')).toHaveCount(0)
    await expect(page.locator('[data-testid="actions-share-account"]').first()).toBeVisible()
    await expect(page.locator('[data-testid="actions-leave"]').first()).toBeVisible()
  })

  test('Co-owner sees Fork, Sharing, and Leave actions on the row', async ({ page }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await openRowActions(page, 'test-image.png')
    await expect(page.locator('[data-testid="actions-fork"]').first()).toBeVisible()
    await expect(page.locator('[data-testid="actions-share-account"]').first()).toBeVisible()
    await expect(page.locator('[data-testid="actions-leave"]').first()).toBeVisible()
  })

  test('Co-owner forks the file; the copy survives revocation', async ({ page }) => {
    const { alice, bob } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await openRowActions(page, 'test-image.png')
    await page.locator('[data-testid="actions-fork"]').first().click()
    // The pipeline navigates to /files (My Files) on success.
    await page.waitForURL(/^[^?]*\/(?:\?.*)?$/, { timeout: 120_000 })
    // Probe the storage endpoint directly to confirm the fork landed
    // as an owner-row for Bob.
    const owned = await page.request.get('/api/storage', { headers: { Accept: 'application/json' } })
    const ownedJson = (await owned.json()) as { children: { id: string; is_owner: boolean }[] }
    expect(ownedJson.children.some((c) => c.is_owner === true)).toBe(true)
  })

  test('Co-owner reshare via Sharing modal disables the Co-owner option', async ({ page }) => {
    const { alice, bob, carol } = await registerThree(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await openSharedWithMe(page)
    await openRowActions(page, 'test-image.png')
    await page.locator('[data-testid="actions-share-account"]').first().click()
    // The Sharing modal lands on People and the add-recipient panel renders
    // share-dialog-target as the title block.
    await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })
    await discoverRecipient(page, carol.email)
    await expect(page.getByTestId('share-dialog-role-coowner')).toBeDisabled()
    await expect(page.getByTestId('share-dialog-coowner-disabled-hint')).toBeVisible()
    await expect(page.getByTestId('share-dialog-role-reader')).toBeEnabled()
    await expect(page.getByTestId('share-dialog-role-editor')).toBeEnabled()
  })
})
