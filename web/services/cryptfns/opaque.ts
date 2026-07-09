import {
  init,
  opaque_client_registration_start,
  opaque_client_registration_finish,
  opaque_client_login_start,
  opaque_client_login_finish
} from './wasm'

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
  const json = opaque_client_registration_finish(state, response, password)

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
 * the envelope.
 */
export async function clientLoginFinish(
  state: string,
  response: string,
  password: string
): Promise<{ finalization: string; sessionKey: string; exportKey: string }> {
  await init()
  const json = opaque_client_login_finish(state, response, password)

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
