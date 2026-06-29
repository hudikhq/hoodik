import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { discoverRecipient, openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

test.describe('Unified Sharing modal: People + Link tabs', () => {
  test('owner adds a recipient on People, creates a link on Link, then revokes the recipient', async ({
    page
  }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await expect(page.getByTestId('file-row-test-image.png')).toBeVisible()

    // People tab — add Bob as Reader. Modal closes after a successful
    // submit so the recipients list is inspected by reopening.
    await openShareDialogFor(page, 'test-image.png')
    await expect(page.getByTestId('sharing-modal-tab-people')).toContainText('People')
    await expect(page.getByTestId('sharing-modal-tab-link')).toContainText('Public link')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 15_000 })

    // Reopen — the People tab now shows Bob, switch to Link tab and
    // create a public link without leaving the unified modal.
    await openShareDialogFor(page, 'test-image.png')
    await expect(page.getByTestId('sharing-modal-tab-people')).toContainText('1')
    const bobRow = page.locator('[data-testid^="sharing-modal-people-row-"]')
    await expect(bobRow.first()).toContainText(bob.email)
    await page.getByTestId('sharing-modal-tab-link').click()
    await page.getByTestId('sharing-link-create').click()
    await expect(page.locator('input[name="link"]')).toHaveValue(/.+/, { timeout: 15_000 })
    await expect(page.getByTestId('sharing-modal-tab-link')).toContainText('1')

    // Revoke Bob from the People tab; the public link tab keeps the link.
    await page.getByTestId('sharing-modal-tab-people').click()
    const revokeRoot = bobRow.first().locator('[data-testid^="sharing-modal-revoke-"]')
    // BaseButtonConfirm — first click swaps to a confirm/cancel pair,
    // second click on the confirm button emits the revoke.
    await revokeRoot.locator('button').first().click()
    await revokeRoot.locator('button').first().click()
    await expect(page.getByTestId('sharing-modal-tab-people')).toContainText('0', {
      timeout: 15_000
    })
    await page.getByTestId('sharing-modal-tab-link').click()
    await expect(page.locator('input[name="link"]')).toHaveValue(/.+/)
  })
})
