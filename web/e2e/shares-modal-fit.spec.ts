import { test, expect } from '@playwright/test'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { closeOpenModal, discoverRecipient } from './helpers/shares'

async function registerTwo(page: Parameters<typeof createUser>[0]) {
  const alice = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  const bob = await createUser(page, randomEmail(), randomPassword())
  await logout(page)
  return { alice, bob }
}

async function openFolderSharingModal(
  page: Parameters<typeof createUser>[0],
  folderName: string
): Promise<void> {
  await closeOpenModal(page)
  await page.getByTestId(`file-row-${folderName}`).locator('[name="actions-dropdown"]').click()
  await page.locator('[data-testid="actions-share-account"]').first().click()
  await expect(page.getByTestId('share-dialog-target')).toBeVisible({ timeout: 15_000 })
}

test.describe('Sharing modal: viewport fit and scrollability', () => {
  test('at 720p viewport with a recipient + role picker the submit button is reachable', async ({
    page
  }) => {
    // Tightest reasonable laptop. Before the compaction + scroll fix the
    // owner-row, role picker, "Allow them to add new files" checkbox, and
    // submit button were all below the viewport with no way to scroll the
    // modal body.
    await page.setViewportSize({ width: 1440, height: 720 })

    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)

    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('fit-folder')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-fit-folder')).toBeVisible({ timeout: 15_000 })

    await openFolderSharingModal(page, 'fit-folder')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-editor').check()

    // The modal node anchors the floating card. Its bounding box must fit
    // within the viewport — and if it doesn't, the inner body must scroll
    // so the user can reach the submit button.
    const submit = page.getByTestId('share-dialog-submit')
    await expect(submit).toBeVisible()
    await submit.scrollIntoViewIfNeeded()
    await expect(submit).toBeInViewport()

    // The reverse-direction check: confirm the title bar is still visible
    // after scrolling, i.e. it stays sticky at the top of the scroll body.
    await expect(page.getByTestId('sharing-modal-close')).toBeVisible()

    // Direct assertion that the modal body is scrollable when its content
    // exceeds available height — `scrollHeight > clientHeight` is the
    // standard browser signal for "this container has overflow content".
    const scrollableMetrics = await page.evaluate(() => {
      const scrollers = Array.from(document.querySelectorAll('.overflow-y-auto'))
      for (const scroller of scrollers) {
        if (!scroller.querySelector('[data-testid="share-dialog-target"]')) continue
        return {
          scrollHeight: scroller.scrollHeight,
          clientHeight: scroller.clientHeight,
          overflowY: getComputedStyle(scroller).overflowY
        }
      }
      return null
    })
    expect(scrollableMetrics).not.toBeNull()
    expect(scrollableMetrics!.overflowY).toBe('auto')
    expect(scrollableMetrics!.scrollHeight).toBeGreaterThanOrEqual(scrollableMetrics!.clientHeight)
  })

  test('at 1080p+ viewport the modal fits without scrolling', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 })

    const { alice, bob } = await registerTwo(page)
    await loginAsUser(page, alice.email, alice.password)

    await page.locator('[name="create-dir"]').click()
    await page.locator('#name').fill('fit-folder-hd')
    await page.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByTestId('file-row-fit-folder-hd')).toBeVisible({ timeout: 15_000 })

    await openFolderSharingModal(page, 'fit-folder-hd')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()

    // The submit button is visible without needing to scroll. We do not
    // assert "scrollHeight == clientHeight" because subpixel rendering can
    // push that off by one even when nothing is clipped.
    await expect(page.getByTestId('share-dialog-submit')).toBeInViewport()
    await expect(page.getByTestId('sharing-modal-close')).toBeInViewport()
  })
})
