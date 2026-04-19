import type { Page } from '@playwright/test'
import { expect } from '@playwright/test'
import { createUser, randomEmail, randomPassword, logout, loginAsUser } from './auth'

/**
 * E2E helpers for the markdown editor and version-history UI.
 *
 * All selectors match the component surface today:
 *   - `create-file` toolbar button + "Create a new file" modal in the browser
 *   - `New Note` button on the /notes landing view
 *   - `md-save`, `md-history`, `md-save` indicator on the editor toolbar
 *   - `vh-restore`, `vh-fork`, `vh-preview`, `vh-delete` in the version sidebar
 *
 * If the UI surface changes, update these helpers — the specs should stay
 * flow-focused, not selector-focused.
 */

/**
 * Register a fresh account AND leave the user authenticated in a way
 * that survives `page.reload()`.
 *
 * The default `createUser` leaves auth purely in-memory — a reload or
 * fresh `page.goto('/notes')` bounces to /auth/login because the Pinia
 * store is rebuilt empty and `maybeCouldMakeRequests()` returns false
 * (no Remember Me key in localStorage).
 *
 * Tests that reload the page or navigate via `page.goto` need the
 * Remember Me token. We get it by logging out + logging back in with
 * the "Remember me" checkbox checked.
 */
export async function createPersistedUser(
  page: Page
): Promise<{ email: string; password: string }> {
  const email = randomEmail()
  const password = randomPassword()
  await createUser(page, email, password)
  // Drop the session cookies…
  await logout(page)
  // …then log back in with Remember Me so the private key is stored in
  // localStorage. Reloads restore the store via `maybeCouldMakeRequests`.
  await page.goto('/auth/login')
  await page.locator('#email').fill(email)
  await page.locator('#password').fill(password)
  // The Remember-Me checkbox is an AppCheckbox rendered as `<input id="remember">`.
  await page.locator('#remember').check()
  await page.getByRole('button', { name: 'Login' }).click()
  await page.waitForURL('**/')
  return { email, password }
}

/**
 * Re-use a logged-out account and return to logged-in state. Paired with
 * `createPersistedUser` for conflict tests that need two separate
 * contexts on the same account.
 */
export async function loginAsPersistedUser(
  page: Page,
  email: string,
  password: string
): Promise<void> {
  await loginAsUser(page, email, password)
}

/**
 * Pick the `<input name="name">` from the currently-visible modal. The
 * file-browser layout mounts CreateDirectoryModal and CreateFileModal
 * side-by-side — both name their input `name`. Scoping to `:visible`
 * keeps us focused on the one the user just opened.
 */
function visibleNameInput(page: Page) {
  return page.locator('input[name="name"]:visible')
}

/**
 * Click the "New file" button in the file-browser toolbar, fill the
 * prompt with the given note name, and wait until the editor is mounted.
 */
export async function createNoteFromBrowser(page: Page, name: string): Promise<string> {
  await page.locator('[name="create-file"]').click()
  const input = visibleNameInput(page)
  await input.waitFor({ state: 'visible' })
  await input.fill(name)
  await page.getByRole('button', { name: 'Create', exact: true }).click()

  // On success the browser navigates to /notes/:id — assert this before
  // caller starts interacting with the editor.
  await page.waitForURL(/\/notes\/[0-9a-f-]{36}/, { timeout: 15_000 })
  await page.locator('.milkdown-wrapper, .md-raw-wrapper').first().waitFor({ state: 'visible' })

  // Extract the note id from the URL for the caller.
  const url = new URL(page.url())
  const id = url.pathname.split('/').pop() as string
  return id
}

/**
 * Create a note from the /notes landing view. Caller must already be on
 * a route inside the authenticated layout (e.g. the file browser) so
 * the SPA's in-memory auth state is intact — a hard `page.goto('/notes')`
 * from a fresh session with no rememberMe would bounce to login.
 */
export async function createNoteFromLanding(page: Page, name: string): Promise<string> {
  // Use the sidebar link to navigate — SPA navigation keeps the Pinia
  // stores alive. The sidebar entry is a router-link with label "Notes".
  await page.locator('aside').getByText('Notes', { exact: true }).first().click()
  await page.waitForURL(/\/notes(\/|$|\?)/)
  await page.getByRole('heading', { name: 'Notes' }).waitFor({ state: 'visible' })

  await page.getByRole('button', { name: 'New Note', exact: true }).click()
  const input = visibleNameInput(page)
  await input.waitFor({ state: 'visible' })
  await input.fill(name)
  await page.getByRole('button', { name: 'Create', exact: true }).click()

  await page.waitForURL(/\/notes\/[0-9a-f-]{36}/, { timeout: 15_000 })
  await page.locator('.milkdown-wrapper, .md-raw-wrapper').first().waitFor({ state: 'visible' })

  const url = new URL(page.url())
  return url.pathname.split('/').pop() as string
}

/**
 * Toggle the editor into raw-markdown mode so we can type plain text
 * without Milkdown wrapping it in nodes. Wait until the raw textarea is
 * actually visible before returning.
 *
 * The view-toggle button label swaps between "Raw markdown" (while in
 * WYSIWYG, click to enter raw) and "WYSIWYG editor" (while in raw, click
 * to exit) — so the menu option alone can't tell us which mode we're
 * in. We probe the textarea instead.
 */
export async function openRawMarkdown(page: Page): Promise<void> {
  const raw = page.locator('.md-raw-textarea')
  // Wait for the editor wrapper to be present.
  await page.locator('.milkdown-wrapper, .md-raw-wrapper').first().waitFor({
    state: 'visible',
    timeout: 10_000
  })

  // Route changes (fork, restore-as-new-note) remount the editor against
  // the new file id and fire background chunk fetches. If we probe mode
  // before those settle we can see a stale wrapper from the outgoing note
  // or click the menu on a remounting component. `networkidle` blocks
  // until 500 ms pass with no in-flight requests — enough for the new
  // note's content to land and the toolbar to stabilise.
  await page.waitForLoadState('networkidle', { timeout: 15_000 })

  // Also wait for the actions button to exist on the *current* toolbar
  // (not a stale one from the previous route) before reading mode.
  const actions = page.locator('[name="md-actions"]')
  await actions.waitFor({ state: 'visible', timeout: 10_000 })

  if (await raw.isVisible().catch(() => false)) return

  await actions.click()
  // Only click when the WYSIWYG-mode menu item is actually present. In
  // raw mode the menu shows "WYSIWYG editor" instead — we'd already be
  // raw, so this branch should never fire.
  const toRaw = page.getByRole('button', { name: 'Raw markdown', exact: true })
  await toRaw.waitFor({ state: 'visible', timeout: 10_000 })
  await toRaw.click()
  await raw.waitFor({ state: 'visible' })
}

/**
 * Replace the raw-markdown textarea content. The editor is already in
 * raw mode from `openRawMarkdown`.
 */
export async function typeRawMarkdown(page: Page, content: string): Promise<void> {
  const raw = page.locator('.md-raw-textarea')
  await raw.click()
  // Clear + type. Using `fill` is what triggers the `input` event the
  // editor listens on to mark the document dirty.
  await raw.fill(content)
}

/**
 * Trigger an explicit save via the save button in the toolbar, then
 * wait until the status flips to "Saved".
 */
export async function saveViaButton(page: Page): Promise<void> {
  await page.locator('[name="md-save"]').click()
  await expect(page.locator('.md-save-status')).toHaveText(/Saved/i, { timeout: 10_000 })
}

/**
 * Wait for the auto-save indicator to reach "Saved". The composable's
 * debounce is 5 s, so we give the timeout ~15 s of headroom.
 */
export async function waitForAutoSaveToSettle(page: Page): Promise<void> {
  await expect(page.locator('.md-save-status')).toHaveText(/Saved/i, { timeout: 15_000 })
}

/**
 * Open the version-history sidebar. Safe to call multiple times — if
 * the panel is already open, this is a no-op.
 */
export async function openHistory(page: Page): Promise<void> {
  const panel = page.locator('.vh-panel')
  if (await panel.isVisible()) return
  await page.locator('[name="md-history"]').click()
  await panel.waitFor({ state: 'visible' })
}

/**
 * Count how many historical versions the server is reporting back in the
 * sidebar. Used by tests that assert "v1 and v2 are now historical".
 */
export async function historyCount(page: Page): Promise<number> {
  const items = page.locator('.vh-panel .vh-item')
  // If there are zero items, the empty message is shown instead; resolve
  // that case as 0.
  if (!(await items.first().isVisible().catch(() => false))) return 0
  return items.count()
}
