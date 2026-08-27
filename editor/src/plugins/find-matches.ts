/**
 * Plaintext (non-regex) search offsets. Empty query yields no matches.
 * Matches are non-overlapping and reported as [from, to) indices.
 */
export interface TextMatch {
  from: number
  to: number
}

export function findMatchOffsets(
  text: string,
  query: string,
  caseSensitive = false,
): TextMatch[] {
  if (!query) return []

  const matches: TextMatch[] = []

  if (caseSensitive) {
    let from = 0
    while (from <= text.length - query.length) {
      const idx = text.indexOf(query, from)
      if (idx === -1) break
      matches.push({ from: idx, to: idx + query.length })
      from = idx + query.length
    }
    return matches
  }

  // Lowercasing is safe for indices when it does not change string length
  // (the usual ASCII / most-Latin case). Rare Unicode expansions (ß → ss)
  // fall back to a code-unit walk on the original strings.
  const haystack = text.toLowerCase()
  const needle = query.toLowerCase()
  if (haystack.length === text.length && needle.length === query.length) {
    let from = 0
    while (from <= haystack.length - needle.length) {
      const idx = haystack.indexOf(needle, from)
      if (idx === -1) break
      matches.push({ from: idx, to: idx + query.length })
      from = idx + needle.length
    }
    return matches
  }

  const needleLen = query.length
  for (let i = 0; i <= text.length - needleLen; ) {
    const slice = text.slice(i, i + needleLen)
    if (slice.toLowerCase() === needle) {
      matches.push({ from: i, to: i + needleLen })
      i += needleLen
    } else {
      i += 1
    }
  }
  return matches
}
