export interface UnsecureChangePassword extends ChangePassword {
  use_private_key: boolean
  private_key: string
}

export interface ChangePassword {
  current_password?: string
  signature?: string
  token?: string
  email: string
  encrypted_private_key: string
  password: string
}
