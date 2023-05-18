import { spawn, Thread, Worker } from 'threads'
import { encrypt as aesEncrypt, decrypt as aesDecrypt } from './aes'
import * as logger from '!/logger'
import {
  ENABLE_CRYPTO_WORKERS,
  MAX_CRYPTO_WORKERS,
  MAX_WAIT_FOR_CRYPTO_WORKER_MS
} from '!/storage/constants'
