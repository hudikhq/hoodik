/**
 * Tracks whether the post-migration notice still needs to be shown for an
 * account. The flag is set only when the migration ceremony actually runs, so
 * natively-registered v2 accounts never see the notice. It persists in
 * localStorage so a reload before acknowledgement doesn't lose it, and clears
 * once the user acknowledges — per browser, which is the right grain for a
 * "here's your recovery key" reminder.
 */
const KEY_PREFIX = 'hoodik:migrationNotice:'

function key(userId: string): string {
  return `${KEY_PREFIX}${userId}`
}

export function markPending(userId: string): void {
  try {
    localStorage.setItem(key(userId), 'pending')
  } catch {
    // A private-mode browser with no storage just skips the notice.
  }
}

export function isPending(userId: string): boolean {
  try {
    return localStorage.getItem(key(userId)) === 'pending'
  } catch {
    return false
  }
}

export function acknowledge(userId: string): void {
  try {
    localStorage.removeItem(key(userId))
  } catch {
    // Nothing to clear.
  }
}
