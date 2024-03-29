/**
 * Size of a single chunk when chunking files.
 * This is used only once the file is being created, the file will
 * be split into chunks of this size. And the number of chunks will
 * be stored in the database. After that, for uploading or downloading the
 * file that number of chunks will be used to calculate that files chunk.
 *
 * Meaning: Its safe to change this after files have been uploaded,
 * but it has to be <= the CHUNK_SIZE_BYTES on the backend, otherwise
 * the request will be denied due to the size of the body.
 */
export const CHUNK_SIZE_BYTES = 1024 * 1024 * 4

/**
 * Enable or disable offloading the crypto operations while
 * uploading and downloading files (in UPLOAD and DOWNLOAD workers)
 * to the CRYPTO workers.
 *
 * This is useful because the crypto operations are blocking and quite
 * CPU intensive, so this way we can offload them to other workers
 * so they don't block the ones that are waiting to encrypt data.
 *
 * @experimental
 */
export const ENABLE_CRYPTO_WORKERS = false

/**
 * The maximum number of workers to spawn for the crypto jobs
 *
 * This does not include the main two workers that are always running (UPLOAD + DOWNLOAD)
 *
 * Should be >= (CONCURRENT_CHUNKS_UPLOAD + 1) otherwise the upload will block... thats why its still
 * experimental
 */
export const MAX_CRYPTO_WORKERS = 8

/**
 * Maximum amount of time to wait for a worker to respond
 * before throwing an error
 */
export const MAX_WAIT_FOR_CRYPTO_WORKER_MS = 10000

/**
 * Upload constants
 */
export const MAX_UPLOAD_RETRIES = 3

/**
 * Uploading multiple chunks at once, this is only done when
 * the web worker is uploading your file.
 */
export const CONCURRENT_CHUNKS_UPLOAD = 8

/**
 * Number of files that will be running the upload at the same time,
 * during testing, best outcome was having only one file running.
 */
export const FILES_UPLOADING_AT_ONE_TIME = 1

/**
 * How long will the finished upload be kept in the status bar
 */
export const KEEP_FINISHED_UPLOADS_FOR_MINUTES = 15

/**
 * Download constants
 */
export const DOWNLOAD_POOL_LIMIT = 16

/**
 * Number of files that will be running the upload at the same time,
 * during testing, best outcome was having only one file running.
 */
export const FILES_DOWNLOADING_AT_ONE_TIME = 1

/**
 * How long will the finished download be kept in the status bar
 */
export const KEEP_FINISHED_DOWNLOADS_FOR_MINUTES = 15

/**
 * Preview constants
 */
export const PREVIEW_MIME_TYPES = ['image/jpg', 'image/jpeg', 'image/png', 'image/gif', 'image/bmp']
export const IMAGE_THUMBNAIL_SIZE_PX = 200
