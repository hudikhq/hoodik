import { mdiLock, mdiThemeLightDark } from '@mdi/js'

export interface NavBarItem {
  icon?: string
  label?: string
  isCurrentUser?: boolean
  isDesktopNoLabel?: boolean
  isToggleLightDark?: boolean
  isDivider?: boolean
  to?: string
  isLogout?: boolean
  isUpload?: boolean
  isCreateDirectory?: boolean
  menu?: NavBarItem[]
  [key: string]: any
}

export default [
  {
    icon: mdiThemeLightDark,
    label: 'Light/Dark',
    isDesktopNoLabel: true,
    isToggleLightDark: true
  },
  {
    icon: mdiLock,
    label: 'Lock',
    isDesktopNoLabel: true,
    to: '/auth/lock'
  }
] as NavBarItem[]
