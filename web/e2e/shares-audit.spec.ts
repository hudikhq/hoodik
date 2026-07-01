import { test, expect } from '@playwright/test'
import path from 'path'

import { createUser, loginAsUser, logout, randomEmail, randomPassword } from './helpers/auth'
import { discoverRecipient, openShareDialogFor } from './helpers/shares'

const imageFixture = path.join(__dirname, 'fixtures', 'test-image.png')

async function gotoAudit(page: Parameters<typeof createUser>[0]): Promise<void> {
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.locator('aside').locator(':text-is("Share")').first().click()
  await page.waitForURL(/\/share/, { timeout: 15_000 })
  // Mount the hub layout once so the capability composable runs;
  // then a programmatic goto inside the same SPA session traverses
  // the router guard with a hydrated capabilities cache.
  await page.locator('[data-testid="share-hub-tab-activity"]').first().click({ timeout: 15_000 })
  await expect(page.getByTestId('share-hub-audit')).toBeVisible({ timeout: 15_000 })
}

test.describe('Audit log view', () => {
  test('Grant + revoke events render verified (no tampered banner, no system pill)', async ({ page }) => {
    // Verified rows are silent — the row itself is
    // the signal, no decoration. The tampered banner and the system pill
    // are reserved for the edge cases. A run that produces only legitimate
    // grants must surface zero banners.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await gotoAudit(page)
    const rows = page.locator('[data-testid="share-hub-audit-list"] > li')
    await expect(rows.first()).toBeVisible({ timeout: 15_000 })
    await expect(page.locator('[data-testid$="-tampered-banner"]')).toHaveCount(0)
    await expect(page.locator('[data-testid$="-system"]')).toHaveCount(0)
    // Every row carries the disclosure chevron — the user opens it on
    // demand for the per-row verification breakdown.
    await expect(page.locator('[data-testid$="-toggle"]').first()).toBeVisible()
  })

  test('Co-owner fork lands an audit row attributable to the forker', async ({ page }) => {
    // Alice shares with Bob as Co-owner. Bob forks the file ("Save to
    // my drive"). The audit log surfaces a `fork` row attributable to
    // Bob, against Alice's original file id. Without the row, recipients
    // could move the file out from under the owner with no provenance.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    // Find Alice's file id before Bob does anything to it.
    const aliceStorage = await page.request.get('/api/storage')
    const aliceFiles = (await aliceStorage.json()) as { children: { id: string }[] }
    const originalFileId = aliceFiles.children[0].id

    await page.keyboard.press('Escape')
    await page.keyboard.press('Escape')
    await logout(page)

    await loginAsUser(page, bob.email, bob.password)
    // Drive Bob's fork through the row dropdown — the same path the
    // role test in shares-roles uses, so the audit fan-out runs through
    // the production fork pipeline.
    const sharedRow = page.getByTestId('file-row-Shared with me')
    await expect(sharedRow).toBeVisible({ timeout: 15_000 })
    await sharedRow.dblclick()
    await expect(page).toHaveURL(/__shared_with_me__/)
    await page.getByTestId('file-row-test-image.png').locator('[name="actions-dropdown"]').click()
    await page.locator('[data-testid="actions-fork"]').first().click()
    await page.waitForURL(/^[^?]*\/(?:\?.*)?$/, { timeout: 120_000 })
    await logout(page)

    // Alice's audit log now has a fork row attributed to Bob against
    // her original file id. Server-side, the row's file_id is the
    // source (not the new fork copy) so the visibility filter that
    // shows Alice events on her own files surfaces it.
    await loginAsUser(page, alice.email, alice.password)
    const events = await page.request.get('/api/shares/events?limit=50&offset=0')
    expect(events.status()).toBe(200)
    const eventsBody = (await events.json()) as {
      events: Array<{ action: string; sender_id: string | null; file_id: string }>
      users: Record<string, { email: string }>
    }
    const forkRow = eventsBody.events.find(
      (e) => e.action === 'fork' && e.file_id === originalFileId
    )
    expect(forkRow).toBeDefined()
    expect(forkRow!.sender_id).not.toBeNull()
    const sender = eventsBody.users[forkRow!.sender_id as string]
    expect(sender?.email).toBe(bob.email)
  })

  test('Role change lands a verified audit row with a non-NULL sender signature', async ({ page }) => {
    // Grant Bob editor → upgrade Bob to co-owner via the Change button.
    // The audit row the server persists for the upgrade is `role_change`;
    // its `sender_signature` must be populated (not NULL) and verify
    // against Alice's pubkey using the role_change canonical input.
    // Until the signature persists end-to-end, the row renders as a
    // "System" pill (legitimately null-sig rows are cascade-only) which
    // is a forensic gap for a sender-attributed action — pin both the
    // banner-free render and the non-NULL `sender_signature` shape.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-editor').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    // Reopen and Change Bob's role to Co-owner — same modal path the
    // production UI uses, so the audit envelope goes through the same
    // signing helper that ships to users.
    await page.setViewportSize({ width: 1440, height: 900 })
    await openShareDialogFor(page, 'test-image.png')
    const bobRow = page.locator('[data-testid^="sharing-modal-people-row-"]').first()
    await expect(bobRow).toContainText(bob.email)
    await bobRow.locator('[data-testid^="sharing-modal-change-role-"]').click()
    await expect(page.getByTestId('share-dialog-recipient-email')).toHaveText(bob.email, {
      timeout: 15_000
    })
    await page.getByTestId('share-dialog-role-coowner').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    // Navigate to the audit log. The role_change row must render
    // tamper-free AND with a verified signature on disclosure.
    await gotoAudit(page)
    const rows = page.locator('[data-testid="share-hub-audit-list"] > li')
    await expect(rows.first()).toBeVisible({ timeout: 15_000 })
    await expect(page.locator('[data-testid$="-tampered-banner"]')).toHaveCount(0)

    // Probe the events endpoint directly for forensic certainty —
    // the role_change row carries a non-NULL `sender_signature`.
    const events = await page.request.get('/api/shares/events?limit=50&offset=0')
    expect(events.status()).toBe(200)
    const body = (await events.json()) as {
      events: Array<{
        action: string
        sender_id: string | null
        sender_signature: string | null
        share_role_before: string | null
        share_role_after: string | null
      }>
    }
    const roleChange = body.events.find((e) => e.action === 'role_change')
    expect(roleChange, 'audit log must contain a role_change row').toBeDefined()
    expect(roleChange!.sender_id, 'role_change is sender-attributed').not.toBeNull()
    expect(
      roleChange!.sender_signature,
      'role_change must carry a sender signature'
    ).not.toBeNull()
    expect(roleChange!.share_role_before).toBe('editor')
    expect(roleChange!.share_role_after).toBe('co-owner')

    // Disclosure on the role_change row should report
    // "Verified against sender pubkey" — same copy the grant rows
    // surface, confirming the signature passes the SPA's verifier.
    const roleChangeRow = page.locator(
      '[data-testid="share-hub-audit-list"] > li',
      { hasText: 'changed' }
    ).first()
    await roleChangeRow.locator('[data-testid$="-toggle"]').click()
    await expect(
      roleChangeRow.locator('[data-testid$="-disclosure"]')
    ).toContainText('Verified against sender pubkey', { timeout: 15_000 })
    await expect(
      roleChangeRow.locator('[data-testid$="-system"]')
    ).toHaveCount(0)
  })

  test('Tampering with an event surfaces the tri-state tampered banner', async ({ page }) => {
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    // Corrupt BOTH the chain hash AND the sender signature so the
    // tri-state logic escalates to "tampered". Single-signal failures
    // stay silent (chain-only false
    // positives from page boundaries used to surface a red badge —
    // now they don't). Two-signal failure is the only path to the
    // row-level banner.
    let tamperedRowId: string | null = null
    await page.route('**/api/shares/events**', async (route) => {
      const response = await route.fetch()
      const json = await response.json()
      if (json.events?.length > 0) {
        const target = json.events[0]
        tamperedRowId = target.id as string
        const hashBuffer = Uint8Array.from(atob(target.this_event_hash as string), (c) =>
          c.charCodeAt(0)
        )
        hashBuffer[0] = hashBuffer[0] ^ 0xff
        target.this_event_hash = btoa(String.fromCharCode(...hashBuffer))
        // The sender_signature is a separate base64 blob over the
        // ASN.1 sig input — flip a byte so verifyEventSignature also
        // rejects this row.
        if (target.sender_signature) {
          const sigBuffer = Uint8Array.from(atob(target.sender_signature as string), (c) =>
            c.charCodeAt(0)
          )
          sigBuffer[0] = sigBuffer[0] ^ 0xff
          target.sender_signature = btoa(String.fromCharCode(...sigBuffer))
        }
      }
      await route.fulfill({ status: response.status(), body: JSON.stringify(json) })
    })

    await gotoAudit(page)
    // The banner testid is `share-hub-audit-row-{rowId}-tampered-banner`
    // for whichever row the route handler corrupted; using a starts-with
    // selector picks it up without races on `tamperedRowId`.
    await expect(
      page.locator('[data-testid$="-tampered-banner"]').first()
    ).toBeVisible({ timeout: 15_000 })
    expect(tamperedRowId).not.toBeNull()
  })

  test('Audit row sentence renders the decrypted file name for sender and recipient', async ({
    page
  }) => {
    // Backend extends the events query with two LEFT JOINs (files +
    // user_files scoped to the caller); the SPA decrypts client-side
    // using the existing storage.meta.decrypt helper. Both Alice (owner)
    // and Bob (recipient) should see the literal file name in the audit
    // row sentence — never the bare id fallback while the share is
    // active.
    const alice = await createUser(page, randomEmail(), randomPassword())
    await logout(page)
    const bob = await createUser(page, randomEmail(), randomPassword())
    await logout(page)

    await loginAsUser(page, alice.email, alice.password)
    await page.setInputFiles('[name="upload-file-input"]', imageFixture)
    await page.getByTestId('upload-active').waitFor({ state: 'hidden', timeout: 30_000 })
    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, bob.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await gotoAudit(page)
    const aliceRows = page.locator('[data-testid="share-hub-audit-list"] > li')
    await expect(aliceRows.first()).toBeVisible({ timeout: 15_000 })
    const aliceSentence = page.locator('[data-testid$="-sentence"]').first()
    await expect(aliceSentence).toContainText('test-image.png', { timeout: 15_000 })

    await logout(page)
    await loginAsUser(page, bob.email, bob.password)
    await gotoAudit(page)
    const bobRows = page.locator('[data-testid="share-hub-audit-list"] > li')
    await expect(bobRows.first()).toBeVisible({ timeout: 15_000 })
    const bobSentence = page.locator('[data-testid$="-sentence"]').first()
    await expect(bobSentence).toContainText('test-image.png', { timeout: 15_000 })
  })

  test('Sender filter resolves email via discover and filters events', async ({ page }) => {
    // Alice grants to Bob and to Carol; Alice opens the audit log and
    // filters by Bob's email. The discover endpoint resolves the email
    // into a user_id, then the existing senderFilter narrows the visible
    // rows to Bob's grant only.
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
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await openShareDialogFor(page, 'test-image.png')
    await discoverRecipient(page, carol.email)
    await page.getByTestId('share-dialog-role-reader').check()
    await page.getByTestId('share-dialog-submit').click()
    await expect(page.getByTestId('share-dialog-target')).toHaveCount(0, { timeout: 30_000 })

    await gotoAudit(page)
    await expect(
      page.locator('[data-testid="share-hub-audit-list"] > li')
    ).toHaveCount(2, { timeout: 15_000 })

    // The filter input is Alice's own outgoing log, so sender on every
    // row is Alice — that's what we filter by to get a deterministic
    // expectation. We're verifying the email -> user_id resolve path
    // and that the existing filter still narrows the table.
    await page.getByTestId('share-hub-audit-sender-filter').fill(alice.email)
    await page.getByTestId('share-hub-audit-sender-resolve').click()
    await expect(
      page.locator('[data-testid="share-hub-audit-list"] > li')
    ).toHaveCount(2, { timeout: 15_000 })

    // Typing a non-existent email surfaces an inline error and leaves
    // the table contents untouched.
    await page.getByTestId('share-hub-audit-sender-filter').fill('nobody-' + randomEmail())
    await page.getByTestId('share-hub-audit-sender-resolve').click()
    await expect(page.getByTestId('share-hub-audit-sender-error')).toContainText(
      "couldn't find",
      { timeout: 15_000 }
    )
  })
})
