import {
  // mdiAccountCircle,
  mdiMonitor
  // mdiGithub,
  // mdiLock,
  // mdiAlertCircle,
  // mdiSquareEditOutline,
  // mdiTable,
  // mdiViewList,
  // mdiTelevisionGuide,
  // mdiResponsive,
  // mdiPalette,
  // mdiReact
} from '@mdi/js'

export interface AsideMenuItemType {
  to: string
  icon: string
  label: string
}

export default [
  {
    to: '/',
    icon: mdiMonitor,
    label: 'Home'
  },
  {
    to: '/files',
    icon: mdiMonitor,
    label: 'Files'
  },
  {
    to: '/auth/login',
    icon: mdiMonitor,
    label: 'Login'
  },
  {
    to: '/auth/register',
    icon: mdiMonitor,
    label: 'Register'
  },
  {
    to: '/auth/register/key',
    icon: mdiMonitor,
    label: 'Private Key'
  },
  {
    to: '/auth/register/two-factor',
    icon: mdiMonitor,
    label: 'Two Factor'
  }
] as AsideMenuItemType[]
