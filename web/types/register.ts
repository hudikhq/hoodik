export interface CreateUser {
  email: string
  password: string
  secret?: string
  token?: string
  pubkey: string
  fingerprint: string
  key_type?: string
  wrapping_pubkey?: string
  encrypted_private_key?: string
  opaque_registration_upload?: string
  invitation_id?: string

  /**
   * Ed25519 identity private key (PKCS#8 PEM), generated on the key screen.
   * Combined with `wrapping_private_key` it forms the recovery bundle the user
   * backs up; neither ever leaves the browser in the clear — both are sealed
   * into `encrypted_private_key` under the OPAQUE export key before the POST.
   */
  identity_private_key?: string
  wrapping_private_key?: string
  /** `v1|ed:<PEM>|x:<PEM>` shown on the key screen for the user to back up. */
  recovery_bundle?: string
  confirm_password?: string
  i_take_all_the_responsibility?: boolean
  store_private_key?: boolean
  i_have_stored_my_private_key?: boolean
}
