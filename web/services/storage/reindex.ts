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
  search_tokens_root: string[]
  search_tokens_file: string[]
}

/**
 * Rebuild the search index for files that predate keyed tags.
 *
 * The re-key migration dropped every old index row, and nothing server-side
 * can rebuild them — the tags are keyed on material only this client holds. So
 * each client walks its own files once.
 *
 * Progress needs no bookkeeping of its own: the server reports a file as
 * pending exactly while it has no root-scope tags, so writing them is what
 * marks it done. Closing the tab, cancelling, or losing the connection all
 * resume from the same place, which is simply "whatever is still pending".
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
   * A note's body is indexed word for word, which is why the old scheme leaked
   * note contents and not just names. Rebuilding that means fetching and
   * decrypting the note — there is no shortcut, the server holds only
   * ciphertext.
   */
  async function textFor(file: AppFile): Promise<string> {
    if (!file.editable || !file.key) {
      return file.name?.toLowerCase() ?? ''
    }

    const bytes = await downloadAndDecrypt(file)

    return new TextDecoder().decode(bytes)
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

    const rootKey = cryptfns.searchRootKey(keypair)
    const fileKey = cryptfns.searchFileKey(file.key)
    const indexed = await textFor(file)

    await Api.put<ReindexRequest, AppFile>(`/api/storage/${file.id}/reindex`, undefined, {
      name_hash: cryptfns.searchTag(rootKey, file.name),
      search_tokens_root: cryptfns.searchTags(rootKey, indexed),
      search_tokens_file: cryptfns.searchTags(fileKey, indexed)
    })
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

    try {
      let pending = await fetchPending()
      total.value = pending.length

      while (pending.length > 0) {
        for (let i = 0; i < pending.length; i += BATCH_SIZE) {
          if (cancelled.value) return

          const batch = pending.slice(i, i + BATCH_SIZE)

          await Promise.all(
            batch.map(async (encrypted) => {
              try {
                await reindexOne(keypair, encrypted)
              } catch {
                failed.value += 1
              } finally {
                done.value += 1
              }
            })
          )
        }

        if (cancelled.value) return

        // The server hands back at most one page at a time, so keep asking
        // until it reports nothing left. Files that failed come back around;
        // if only failures remain, stop rather than spin on them.
        const next = await fetchPending()
        if (next.length >= pending.length) break

        total.value += next.length
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
