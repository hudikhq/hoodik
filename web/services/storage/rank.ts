/**
 * Client-side refinement of server search results.
 *
 * The server can only rank opaque tags; the client holds the plaintext — the
 * raw query, every decrypted name, and the hydrated bodies of candidate
 * notes — so precision lives here. Scoring is tiered so a stronger kind of
 * evidence always beats any amount of a weaker kind: an exact name match
 * outranks every substring match, any name evidence outranks body evidence,
 * and server tag counts only order rows nothing textual separates.
 *
 * The same tiers are implemented in the app's Dart search service; the two
 * share the golden expectations in their test suites the same way the tag
 * vector is shared. Change them together.
 */

export interface RankableRow {
  id: string
  name?: string
  finished_upload_at?: number | string | null
  created_at?: number | string
  search_hits?: number | null
  search_name_hits?: number | null
}

/** Query words the way the index tokenizes: alphanumeric runs, two chars up. */
export function queryWords(query: string): string[] {
  return query
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((word) => word.length >= 2)
}

export function scoreRow(query: string, row: RankableRow, body?: string): number {
  const raw = query.trim().toLowerCase()
  const words = queryWords(query)
  const name = (row.name || '').toLowerCase()

  let score = 0

  if (raw.length > 0 && name === raw) {
    score += 1_000_000
  } else if (raw.length > 0 && name.startsWith(raw)) {
    score += 500_000
  } else if (raw.length > 0 && name.includes(raw)) {
    score += 250_000
  }

  if (words.length > 0) {
    const inName = words.filter((word) => name.includes(word)).length
    if (inName === words.length) {
      score += 100_000
    }
    score += Math.round((10_000 * inName) / words.length)
  }

  if (body) {
    const text = body.toLowerCase()
    if (raw.length > 0 && text.includes(raw)) {
      score += 50_000
    }
    if (words.length > 0) {
      const inBody = words.filter((word) => text.includes(word)).length
      if (inBody === words.length) {
        score += 25_000
      }
      score += Math.round((5_000 * inBody) / words.length)
    }
  }

  if ((row.search_name_hits || 0) > 0) {
    score += 2_000
  }
  score += Math.min(row.search_hits || 0, 99) * 10

  return score
}

/**
 * Order rows by refined score, newest first among ties. [bodies] maps row id
 * to hydrated plaintext for the note candidates that could be loaded; rows
 * without an entry are scored on their name and server evidence alone.
 */
export function rankSearchResults<T extends RankableRow>(
  query: string,
  rows: T[],
  bodies: Map<string, string> = new Map()
): T[] {
  const at = (row: RankableRow) => Number(row.finished_upload_at || row.created_at || 0)

  return [...rows].sort((a, b) => {
    const diff = scoreRow(query, b, bodies.get(b.id)) - scoreRow(query, a, bodies.get(a.id))
    if (diff !== 0) return diff
    return at(b) - at(a)
  })
}
