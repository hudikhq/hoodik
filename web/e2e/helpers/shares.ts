import type { Page } from '@playwright/test'
import { expect } from '@playwright/test'

/**
 * Dismiss any open CardBoxModal by clicking its overlay. The overlay
 * intercepts pointer events for any visible modal in the layout, so the
 * test browser ends up firing a `cancel` exactly like a real user does.
 */
export async function closeOpenModal(page: Page): Promise<void> {
  const overlay = page.locator('div[class*="bg-brownish-900"]').first()
  if (await overlay.isVisible().catch(() => false)) {
    // Press Escape — the CardBoxModal listens for it and emits cancel.
    await page.keyboard.press('Escape')
  }
}

/**
 * Open the SharingModal for a given file row via the row's actions
 * dropdown. DetailsModal no longer exposes a Share affordance — every
 * Share-from-row path now routes through the dropdown's Sharing entry.
 */
export async function openShareDialogFor(page: Page, fileName: string): Promise<void> {
  await closeOpenModal(page)
  await page.getByTestId(`file-row-${fileName}`).locator('[name="actions-dropdown"]').click()
  await page.locator('[data-testid="actions-share-account"]').first().click()
  await expect(page.getByTestId('share-dialog-target')).toBeVisible()
}

/**
 * Fill the recipient email field, click Find user, and wait for the
 * fingerprint row to appear (or the documented error message).
 */
export async function discoverRecipient(page: Page, email: string): Promise<void> {
  await page.locator('input[name="recipient-email"]').fill(email)
  await page.getByTestId('share-dialog-discover').click()
  // Either a recipient resolves and the fingerprint row mounts, or the
  // documented discover-error banner does. Wait for whichever lands first
  // so subsequent assertions don't race the network round-trip.
  await expect(
    page.locator(
      '[data-testid="share-dialog-fingerprint"], [data-testid="share-dialog-discover-error"]'
    ).first()
  ).toBeVisible({ timeout: 10_000 })
}

/**
 * Navigate from a logged-in /files screen into the synthetic
 * "Shared with me" folder. Closes any modal still up from a previous
 * interaction before the row click.
 */
export async function openSharedWithMe(page: Page): Promise<void> {
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  const sharedFolder = page.getByTestId('file-row-Shared with me')
  await expect(sharedFolder).toBeVisible({ timeout: 15_000 })
  await sharedFolder.dblclick()
  await expect(page).toHaveURL(/__shared_with_me__/)
}

/**
 * Click the row-actions dropdown trigger inside the virtual folder so
 * the Fork / Remove / Sharing entries become clickable.
 */
export async function openRowActions(page: Page, fileName: string): Promise<void> {
  const row = page.getByTestId(`file-row-${fileName}`)
  await expect(row).toBeVisible({ timeout: 15_000 })
  await row.locator('[name="actions-dropdown"]').click()
}
