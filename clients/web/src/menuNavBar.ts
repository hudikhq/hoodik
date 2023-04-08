import {
  mdiMenu,
  mdiClockOutline,
  mdiCloud,
  mdiCrop,
  mdiCogOutline,
  mdiLock,
  mdiThemeLightDark
} from '@mdi/js'

export interface NavBarItem {
  icon?: string
  label?: string
  isCurrentUser?: boolean
  isDesktopNoLabel?: boolean
  isToggleLightDark?: boolean
  isDivider?: boolean
  to?: string
  isLogout?: boolean
  menu?: NavBarItem[]
}

export default [
  {
    icon: mdiMenu,
    label: 'Sample menu',
    menu: [
      {
        icon: mdiClockOutline,
        label: 'Item One'
      },
      {
        icon: mdiCloud,
        label: 'Item Two'
      },
      {
        isDivider: true
      },
      {
        icon: mdiCrop,
        label: 'Item Last'
      }
    ]
  },
  {
    isCurrentUser: true,
    menu: [
      {
        icon: mdiCogOutline,
        label: 'Settings',
        to: '/settings'
      }
    ]
  },
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
