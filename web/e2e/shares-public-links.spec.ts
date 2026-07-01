import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, randomEmail, randomPassword } from './helpers/auth'
import { openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

test.describe('Sharing modal: Link tab', () => {
  test('Link tab survives across modal opens; tab badge shows the count', async ({ page }) => {
    await createUser(page, randomEmail(), randomPassword())
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await page.getByTestId('sharing-modal-tab-link').click()
    await page.getByTestId('sharing-link-create').click()

    const linkField = page.locator('input[name="link"]')
    await expect(linkField).toHaveValue(/.+/, { timeout: 15_000 })
    const firstUrl = await linkField.inputValue()
    expect(firstUrl).toBeTruthy()
    // After link creation the badge updates inline against the same
    // links store the People+Link tabs read from.
    await expect(page.getByTestId('sharing-modal-tab-link')).toContainText('1')

    // Close SharingModal via its dedicated close affordance.
    await page.getByTestId('sharing-modal-close').click()
    await expect(page.getByTestId('sharing-modal-tab-link')).toHaveCount(0)

    // Reopen the modal — the link count badge persists because the
    // underlying links store survived the unmount + remount.
    await openShareDialogFor(page, 'test-image.png')
    await expect(page.getByTestId('sharing-modal-tab-link')).toContainText('1')
    await page.getByTestId('sharing-modal-tab-link').click()
    await expect(page.locator('input[name="link"]')).toHaveValue(firstUrl, { timeout: 15_000 })
  })

  test('anonymous download via the public link succeeds without a session', async ({
    page,
    browser
  }) => {
    await createUser(page, randomEmail(), randomPassword())
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await page.getByTestId('sharing-modal-tab-link').click()
    await page.getByTestId('sharing-link-create').click()

    const linkUrl = await page.locator('input[name="link"]').inputValue()
    expect(linkUrl).toBeTruthy()
    expect(linkUrl).toMatch(/#/)

    // A fresh browser context has no Hoodik cookies, so this is a
    // strictly anonymous download — the recipient never authenticates,
    // and the request must stream the file straight through without a
    // bounce to /auth/login.
    const anonContext = await browser.newContext()
    const anonPage = await anonContext.newPage()
    await anonPage.goto(linkUrl)
    await expect(anonPage.locator('img[name="original"]')).toBeVisible({ timeout: 30_000 })
    await expect(anonPage.locator('img[name="original"]')).toHaveAttribute('alt', 'test-image.png')
    // The anonymous page never lands on /auth/login or /auth/register;
    // an HTML bounce would be the canonical kill-switch regression on
    // the public download path.
    await expect(anonPage).not.toHaveURL(/\/auth\//)
    await anonContext.close()
  })
})
