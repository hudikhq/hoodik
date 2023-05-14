import { mdiLock } from '@mdi/js'
import type { RouteLocation } from 'vue-router'

export interface NavBarItem {
  icon?: string
  label?: string
  isCurrentUser?: boolean
  isDesktopNoLabel?: boolean
  isTogglelight?: boolean
  isDivider?: boolean
  to?: RouteLocation
  isLogout?: boolean
  isUpload?: boolean
  isCreateDirectory?: boolean
  menu?: NavBarItem[]
  [key: string]: any
}

export default [
  {
    icon: mdiLock,
    label: 'Lock',
    isDesktopNoLabel: true,
    to: { name: 'lock' }
  }
] as NavBarItem[]
