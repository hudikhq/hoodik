import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import {
  audit_event_encode_v1,
  audit_event_sig_input_encode_v1,
  folder_member_list_encode_v1,
  member_sig_encode_v1,
  share_payload_encode_v1
} from '../../node_modules/transfer/transfer.js'

const FIXTURES_DIR = resolve(__dirname, '../../../hoodik/tests/fixtures')

function fixture(name: string): Uint8Array {
  return new Uint8Array(readFileSync(resolve(FIXTURES_DIR, name)))
}

function repeat(byte: number, length: number): Uint8Array {
  return new Uint8Array(length).fill(byte)
}

describe('cryptfns::asn1 WASM exports', () => {
  it('share_payload_encode_v1 produces the committed fixture bytes', () => {
    const bytes = share_payload_encode_v1(
      repeat(0x11, 16),
      repeat(0x22, 16),
      repeat(0x33, 32),
      1, // Editor
      repeat(0x44, 16),
      repeat(0x55, 32),
      1735689600n,
      repeat(0x66, 16)
    )
    expect(bytes).toBeDefined()
    expect(new Uint8Array(bytes as Uint8Array)).toEqual(fixture('share_request_v1.der'))
  })

  it('member_sig_encode_v1 produces the committed fixture bytes', () => {
    const bytes = member_sig_encode_v1(
      repeat(0x77, 16),
      repeat(0xaa, 270),
      repeat(0x88, 32),
      2, // CoOwner
      1735689700n
    )
    expect(bytes).toBeDefined()
    expect(new Uint8Array(bytes as Uint8Array)).toEqual(fixture('member_sig_v1.der'))
  })

  it('audit_event_encode_v1 produces the committed fixture bytes', () => {
    const bytes = audit_event_encode_v1(
      repeat(0xaa, 16),
      repeat(0xbb, 16),
      repeat(0xcc, 16),
      'grant',
      0, // Reader
      1735689800n
    )
    expect(bytes).toBeDefined()
    expect(new Uint8Array(bytes as Uint8Array)).toEqual(fixture('audit_event_v1.der'))
  })

  it('audit_event_sig_input_encode_v1 produces the committed fixture bytes', () => {
    const bytes = audit_event_sig_input_encode_v1(
      repeat(0xaa, 16),
      repeat(0xbb, 16),
      repeat(0xcc, 16),
      2, // RoleChange
      0, // Reader (before)
      1, // Editor (after)
      1735689900n
    )
    expect(bytes).toBeDefined()
    expect(new Uint8Array(bytes as Uint8Array)).toEqual(
      fixture('audit_event_sig_input_v1.der')
    )
  })

  it('folder_member_list_encode_v1 produces the committed fixture bytes', () => {
    // Mirrors `fixtures::folder_member_list_v1()` in hoodik/tests/fixtures/mod.rs:
    // owner 0x11 (reader+is_owner), co-owner 0x22 signed by owner, editor 0x33
    // signed by co-owner. Members deliberately supplied out of order so the
    // canonical-sort path runs.
    const userIds = new Uint8Array(3 * 16)
    userIds.set(repeat(0x33, 16), 0)
    userIds.set(repeat(0x11, 16), 16)
    userIds.set(repeat(0x22, 16), 32)

    const fingerprints = new Uint8Array(3 * 32)
    fingerprints.set(repeat(0xc3, 32), 0)
    fingerprints.set(repeat(0xa1, 32), 32)
    fingerprints.set(repeat(0xb2, 32), 64)

    const shareRoles = new Uint8Array([1, 0, 2])
    const isOwnerFlags = new Uint8Array([0, 1, 0])

    const signedBy = new Uint8Array(3 * 16)
    signedBy.set(repeat(0x22, 16), 0)
    signedBy.set(repeat(0x11, 16), 16)
    signedBy.set(repeat(0x11, 16), 32)

    const bytes = folder_member_list_encode_v1(
      repeat(0xf0, 16),
      repeat(0x11, 16),
      userIds,
      fingerprints,
      shareRoles,
      isOwnerFlags,
      signedBy,
      1736000000n
    )
    expect(bytes).toBeDefined()
    expect(new Uint8Array(bytes as Uint8Array)).toEqual(
      fixture('folder_member_list_v1.der')
    )
  })
})
