export interface CreateUser {
  email: string
  password: string
  secret?: string
  token?: string
  pubkey: string
  fingerprint: string
  encrypted_private_key?: string
  invitation_id?: string

  /**
   * Optional parameters that are only used for the registration process
   * on the frontend. Private key is not sent to backend server unencrypted
   */
  unencrypted_private_key?: string
  confirm_password?: string
  i_take_all_the_responsibility?: boolean
  store_private_key?: boolean
  i_have_stored_my_private_key?: boolean
}
