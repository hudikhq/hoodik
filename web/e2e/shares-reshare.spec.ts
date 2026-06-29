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

test.describe('Shares: Co-owner re-share', () => {
  test('Co-owner B re-shares to C; C sees the file in their virtual folder', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const carol = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

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
    await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })
    await discoverRecipient(page, carol.email)
    await page.getByTestId('share-dialog-role-reader').check()
    // First contact with Carol from Bob's session — the unknown pill
    // surfaces but the Share button stays enabled.
    await expect(page.getByTestId('share-dialog-unknown')).toBeVisible()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 60_000 })

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)
    await loginAsUser(page, carol.email, carol.password)
    await openSharedWithMe(page)
    const row = page.getByTestId('file-row-test-image.png')
    await expect(row).toBeVisible({ timeout: 15_000 })
    // The row's badge surfaces the file's actual owner — Alice, not the
    // intermediate Co-owner Bob who handed Carol the wrap. Sharing the
    // file shares the roster, and the owner identity is what's load-
    // bearing for the recipient to recognise whose data they hold. Bob's
    // role as granter stays visible in the audit log.
    await expect(row.getByTestId('shared-by-badge')).toContainText(alice.email)
  })
})
