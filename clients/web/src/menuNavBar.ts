import { mdiLock, mdiThemeLightDark, mdiFile, mdiFileCabinet } from '@mdi/js'

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
    sw: true,
    icon: mdiFile,
    label: 'Worker Test'
  },
  {
    label: '',
    menu: [
      {
        icon: mdiFileCabinet,
        label: 'Directory',
        isCreateDirectory: true
      },
      {
        isDivider: true
      },
      {
        icon: mdiFile,
        label: 'Upload',
        isUpload: true
      }
      // {
      //   icon: mdiCrop,
      //   label: 'Upload Folder'
      // }
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
