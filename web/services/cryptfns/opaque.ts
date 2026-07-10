import {
  init,
  opaque_client_registration_start,
  opaque_client_registration_finish,
  opaque_client_login_start,
  opaque_client_login_finish
} from './wasm'

/**
 * Argon2id parameters that shape OPAQUE's `export_key`. `login/start` returns
 * the account's stored values; a registration has none yet, so it seals under
 * these current defaults.
 */
export interface KsfParams {
  m_cost: number
  t_cost: number
  p_cost: number
}

/**
 * The KSF a fresh registration seals under. Must stay byte-identical to the
 * server's `current_ksf_params()` (OWASP Argon2id: 64 MiB / t=3 / p=1) — the
 * server records those same values, and login later re-derives `export_key`
 * from them. Raising the work factor is a coordinated bump on both sides.
 */
export const CURRENT_KSF: KsfParams = { m_cost: 64 * 1024, t_cost: 3, p_cost: 1 }

/**
 * Begin OPAQUE registration; returns the client state to keep and the
 * message to send to the server.
 */
export async function clientRegistrationStart(
  password: string
): Promise<{ state: string; message: string }> {
  await init()
  const json = opaque_client_registration_start(password)

  if (!json) {
    throw new Error('opaque_client_registration_start failed')
  }

  return JSON.parse(json)
}

/**
 * Finish OPAQUE registration against the server's response; returns the
 * final message for the server and the export key used to seal the envelope.
 */
export async function clientRegistrationFinish(
  state: string,
  response: string,
  password: string
): Promise<{ message: string; exportKey: string }> {
  await init()
  const json = opaque_client_registration_finish(
    state,
    response,
    password,
    CURRENT_KSF.m_cost,
    CURRENT_KSF.t_cost,
    CURRENT_KSF.p_cost
  )

  if (!json) {
    throw new Error('opaque_client_registration_finish failed')
  }

  const parsed = JSON.parse(json)
  return { message: parsed.message, exportKey: parsed.export_key }
}

/**
 * Begin OPAQUE login; returns the client state to keep and the message to
 * send to the server.
 */
export async function clientLoginStart(
  password: string
): Promise<{ state: string; message: string }> {
  await init()
  const json = opaque_client_login_start(password)

  if (!json) {
    throw new Error('opaque_client_login_start failed')
  }

  return JSON.parse(json)
}

/**
 * Finish OPAQUE login against the server's credential response; returns the
 * finalization message, the session key, and the export key used to open
 * the envelope. `ksf` is the account's stored KSF from `login/start` — the
 * `export_key` only matches what registration produced if it is stretched with
 * the same parameters.
 */
export async function clientLoginFinish(
  state: string,
  response: string,
  password: string,
  ksf: KsfParams
): Promise<{ finalization: string; sessionKey: string; exportKey: string }> {
  await init()
  const json = opaque_client_login_finish(
    state,
    response,
    password,
    ksf.m_cost,
    ksf.t_cost,
    ksf.p_cost
  )

  if (!json) {
    throw new Error('opaque_client_login_finish failed')
  }

  const parsed = JSON.parse(json)
  return {
    finalization: parsed.finalization,
    sessionKey: parsed.session_key,
    exportKey: parsed.export_key
  }
}
