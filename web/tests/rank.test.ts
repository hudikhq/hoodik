import { describe, expect, it } from 'vitest'
import { queryWords, rankSearchResults, scoreRow } from '../services/storage/rank'

/**
 * Golden ranking cases, mirrored in the app's `search_rank_test.dart` the
 * same way the cross-client tag vector is. A deliberate change to the tiers
 * regenerates both together.
 */
describe('search result refinement', () => {
  it('UNIT: rank: tokenizes queries like the index does', () => {
    expect(queryWords('IMG_0179.mov')).toEqual(['img', '0179', 'mov'])
    expect(queryWords('a b! š-9')).toEqual([])
  })

  it('UNIT: rank: exact filename beats every text-rich note', () => {
    const video = { id: 'v', name: 'IMG_0179.mov', search_hits: 3, search_name_hits: 3 }
    const note = { id: 'n', name: 'todo.md', search_hits: 40, search_name_hits: 0 }

    const ranked = rankSearchResults('IMG_0179.mov', [note, video])
    expect(ranked.map((r) => r.id)).toEqual(['v', 'n'])
  })

  it('UNIT: rank: name prefix beats a big document with many tag hits', () => {
    const todo = { id: 't', name: 'todo.md', search_hits: 2, search_name_hits: 2 }
    const handoff = { id: 'h', name: 'handoff.md', search_hits: 99, search_name_hits: 0 }

    const ranked = rankSearchResults('todo', [handoff, todo])
    expect(ranked.map((r) => r.id)).toEqual(['t', 'h'])
  })

  it('UNIT: rank: a hydrated body phrase beats a partial name match', () => {
    const config = { id: 'c', name: 'duzluk.md', search_hits: 2, search_name_hits: 0 }
    const panels = { id: 'p', name: 'solar-panels.md', search_hits: 2, search_name_hits: 1 }
    const bodies = new Map([['c', 'notes on the solar inverter wiring and limits']])

    const ranked = rankSearchResults('solar inverter', [panels, config], bodies)
    expect(ranked.map((r) => r.id)).toEqual(['c', 'p'])
  })

  it('UNIT: rank: newer row wins a score tie', () => {
    const older = { id: 'o', name: 'plan.md', created_at: 100 }
    const newer = { id: 'n', name: 'plan.md', created_at: 200 }

    const ranked = rankSearchResults('plan', [older, newer])
    expect(ranked.map((r) => r.id)).toEqual(['n', 'o'])
  })

  it('UNIT: rank: pinned score values shared with the app suite', () => {
    // The exact numbers the tiers produce, pinned so a drift between the
    // web and app implementations fails a test instead of splitting the
    // ranking between clients.
    expect(scoreRow('todo', { id: 'a', name: 'todo.md', search_hits: 2, search_name_hits: 2 })).toBe(
      612020
    )
    expect(
      scoreRow(
        'solar inverter',
        { id: 'b', name: 'duzluk.md', search_hits: 2, search_name_hits: 0 },
        'notes on the solar inverter wiring and limits'
      )
    ).toBe(80020)
  })
})
