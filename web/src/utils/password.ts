import { zxcvbn } from '@zxcvbn-ts/core'

export function isStrongPassword(password: string | undefined): boolean {
  return !!password && zxcvbn(password).score > 3
}
