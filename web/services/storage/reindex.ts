import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

import Api from '../api'
import * as cryptfns from '../cryptfns'
import * as meta from './meta'
import { downloadAndDecrypt } from './download/sync'

import type { AppFile, EncryptedAppFile, KeyPair } from 'types'

/**
 * Files handled per round. Small enough that cancelling feels immediate and
 * that a note download never blocks the bar for long, large enough that the
 * request overhead is not what dominates the sweep.
 */
const BATCH_SIZE = 10

export interface ReindexRequest {
  name_hash: string
  /** Account fingerprint the tags were keyed under. The server rejects a
   *  value that is not the live `users.fingerprint`, so a sweep that started
   *  under a discarded key cannot retire files from the pending list. */
  fingerprint: string
  search_tokens_root: string[]
  search_tokens_file: string[]
  /** Content digests re-keyed under the file's search key, replacing the
   *  bare digests migrated rows still carry. */
  md5?: string
  sha1?: string
  sha256?: string
  blake2b?: string
  /** The tags that make those digests findable in search. Separate from the
   *  word tokens because they land in the digest scopes, which renames never
   *  touch. */
  digest_tokens_root?: string[]
  digest_tokens_file?: string[]
}

/** Bare digest lengths in hex, per algorithm. A keyed tag is 32 hex chars. */
const BARE_DIGEST_LENGTH = { md5: 32, sha1: 40, sha256: 64, blake2b: 128 } as const

type DigestName = keyof typeof BARE_DIGEST_LENGTH

/**
 * The digest columns a sweep may re-key: the ones still holding a bare
 * digest. A file can pass through the sweep twice — a note edited before the
 * sweep already got a keyed sha256 from its save, and a failed
 * crypto-migration ceremony can run the whole sweep once on the old key —
 * and keying a keyed value corrupts the column beyond repair. Shape decides
 * for sha1/sha256/blake2b; bare MD5 is 32 hex chars like a tag, so it goes
 * by its siblings: every writer that stored an MD5 stored a SHA-256 next to
 * it, so a row keying any sibling is a bare row and one keying none is
 * already done.
 */
export function bareDigests(
  file: Partial<Record<DigestName, string>>
): Partial<Record<DigestName, string>> {
  const bare = (name: DigestName): string | undefined => {
    const digest = file[name]
    if (!digest || digest.length !== BARE_DIGEST_LENGTH[name]) return undefined
    return /^[0-9a-f]+$/i.test(digest) ? digest : undefined
  }

  const digests: Partial<Record<DigestName, string>> = {
    sha1: bare('sha1'),
    sha256: bare('sha256'),
    blake2b: bare('blake2b')
  }
  if (digests.sha1 || digests.sha256 || digests.blake2b) {
    digests.md5 = bare('md5')
  }

  return digests
}

/**
 * Rebuild the search index for files that predate keyed tags.
 *
 * The re-key migration dropped every old index row, and nothing server-side
 * can rebuild them — the tags are keyed on material only this client holds. So
 * each client walks its own files once.
 *
 * Progress needs no bookkeeping of its own: the server reports a file as
 * pending exactly while its `name_hash` is the blank the migration left, so
 * the keyed hash every re-index writes is what marks it done. Closing the
 * tab, cancelling, or losing the connection all resume from the same place,
 * which is simply "whatever is still pending".
 */
export const store = defineStore('reindex', () => {
  const total = ref(0)
  const done = ref(0)
  const running = ref(false)
  const background = ref(false)
  const failed = ref(0)

  /**
   * Stop after the batch in flight. Whatever is left is still pending
   * server-side, so the next session picks it up from there.
   */
  const cancelled = ref(false)

  const progress = computed(() => (total.value === 0 ? 0 : Math.round((done.value / total.value) * 100)))
  const visible = computed(() => running.value && !background.value && !cancelled.value)

  async function fetchPending(): Promise<EncryptedAppFile[]> {
    const response = await Api.get<EncryptedAppFile[]>('/api/storage/reindex')

    return response?.body ?? []
  }

  /**
   * How many files still need doing. Cheap enough to call on unlock to decide
   * whether the modal is worth showing at all.
   */
  async function countPending(): Promise<number> {
    return (await fetchPending()).length
  }

  /**
   * A note's body is indexed word for word alongside its name, which is why
   * the old scheme leaked note contents and not just names. Rebuilding that
   * means fetching and decrypting the note — there is no shortcut, the server
   * holds only ciphertext — and the name rides along so a swept note carries
   * the same tokens a saved one does.
   */
  async function textFor(file: AppFile): Promise<string> {
    const name = file.name ?? ''
    if (!file.editable || !file.key) {
      return name
    }

    const bytes = await downloadAndDecrypt(file)

    return `${name}\n${new TextDecoder().decode(bytes)}`
  }

  async function reindexOne(keypair: KeyPair, encrypted: EncryptedAppFile): Promise<void> {
    const privateKey = keypair.wrappingPrivate || keypair.input

    if (!privateKey) {
      throw new Error('Cannot re-index without an unlocked private key')
    }

    const file = { ...encrypted, ...(await meta.decrypt(encrypted, privateKey)) } as AppFile

    if (!file.key || !file.name) {
      throw new Error('file has no key or name')
    }

    const fingerprint = keypair.fingerprint
    if (!fingerprint) {
      throw new Error('Cannot re-index without the account fingerprint')
    }

    const rootKey = cryptfns.searchRootKey(keypair)
    const fileKey = cryptfns.searchFileKey(file.key)
    const indexed = await textFor(file)

    const body: ReindexRequest = {
      name_hash: cryptfns.searchTag(rootKey, file.name),
      fingerprint,
      search_tokens_root: cryptfns.searchTags(rootKey, indexed),
      search_tokens_file: cryptfns.searchTags(fileKey, indexed)
    }

    // Migrated rows still carry bare content digests — the third copy of the
    // same leak. Re-key each from the stored value (no re-download needed)
    // and index it in both scopes, which is what makes pasting a digest into
    // search find the file. Only values still in the bare shape are re-keyed;
    // `bareDigests` above says why.
    const digests = bareDigests(file)

    for (const name of ['md5', 'sha1', 'sha256', 'blake2b'] as const) {
      const digest = digests[name]
      if (!digest) continue
      body[name] = cryptfns.searchTag(fileKey, digest)
      body.digest_tokens_root = body.digest_tokens_root ?? []
      body.digest_tokens_file = body.digest_tokens_file ?? []
      body.digest_tokens_root.push(`${cryptfns.searchTag(rootKey, digest)}:1`)
      body.digest_tokens_file.push(`${cryptfns.searchTag(fileKey, digest)}:1`)
    }

    await Api.put<ReindexRequest, AppFile>(`/api/storage/${file.id}/reindex`, undefined, body)
  }

  /**
   * Walk every pending file in batches until the server reports none left.
   *
   * A file that throws is counted and skipped rather than aborting the sweep —
   * one unreadable file should not cost the user their whole index. It stays
   * pending, so the next run tries it again.
   */
  async function run(keypair: KeyPair): Promise<void> {
    if (running.value) return

    running.value = true
    cancelled.value = false
    background.value = false
    done.value = 0
    failed.value = 0

    // By id rather than by count, and at most one attempt per file per run.
    // The server's answer to an attempt is authoritative either way: success
    // takes the file off the pending list, and a failure leaves it there for
    // the next run. Re-attempting inside this run would change nothing the
    // second time — and if a server ever kept reporting a successfully
    // re-indexed file as pending, retrying it would spin this loop forever.
    const failedIds = new Set<string>()
    const attemptedIds = new Set<string>()
    const seenIds = new Set<string>()

    try {
      let pending = await fetchPending()
      pending.forEach((f) => seenIds.add(f.id))
      total.value = seenIds.size

      while (pending.length > 0) {
        for (let i = 0; i < pending.length; i += BATCH_SIZE) {
          if (cancelled.value) return

          const batch = pending.slice(i, i + BATCH_SIZE)

          await Promise.all(
            batch.map(async (encrypted) => {
              try {
                await reindexOne(keypair, encrypted)
              } catch {
                failedIds.add(encrypted.id)
              } finally {
                attemptedIds.add(encrypted.id)
                done.value += 1
                failed.value = failedIds.size
              }
            })
          )
        }

        if (cancelled.value) return

        // The server pages at 500, so keep asking until nothing fresh comes
        // back. Stopping when the next page is no smaller than this one broke
        // after a single page on any account past that limit: page two is also
        // full, so it read as "no progress" and the sweep quit with most of
        // the account still unindexed. Only what this run has not touched
        // counts as fresh.
        const next = (await fetchPending()).filter((f) => !attemptedIds.has(f.id))
        if (!next.length) break

        next.forEach((f) => seenIds.add(f.id))
        total.value = seenIds.size
        pending = next
      }
    } finally {
      running.value = false
    }
  }

  /** Dismiss the modal and stop. Whatever is left stays pending for next time. */
  function cancel() {
    cancelled.value = true
  }

  /** Dismiss the modal but let the sweep finish. */
  function continueInBackground() {
    background.value = true
  }

  return {
    total,
    done,
    failed,
    running,
    background,
    cancelled,
    progress,
    visible,
    countPending,
    run,
    cancel,
    continueInBackground
  }
})
