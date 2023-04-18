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
    label: 'My Files'
  }
] as AsideMenuItemType[]
