export const gradientBgBase = 'bg-gradient-to-tr'
export const gradientBgPurplePink = `${gradientBgBase} from-greeny-400 via-orangy-500 to-redish-500`
export const gradientBgDark = `${gradientBgBase} from-brownish-700 via-brownish-900 to-brownish-800`
export const gradientBgPinkRed = `${gradientBgBase} from-redish-400 via-redish-500 to-orangy-500`
export type ColorType =
  | 'dark'
  | 'light'
  | 'contrast'
  | 'success'
  | 'danger'
  | 'warning'
  | 'info'
  | 'empty'

export const colorsBgLight = {
  light: 'bg-brownish-100 text-black dark:bg-brownish-900/70 dark:text-white',
  dark: 'bg-brownish-100 text-black dark:bg-brownish-900/70 dark:text-white',
  contrast: 'bg-brownish-800 text-white dark:bg-brownish-100 dark:text-black',
  success: 'bg-greeny-500 border-greeny-500 text-white',
  danger: 'bg-redish-500 border-redish-500 text-white',
  warning: 'bg-orangy-500 border-orangy-500 text-white',
  info: 'bg-redish-500 border-redish-500 text-white',
  empty: 'bg-transparent text-black dark:text-white'
}

export const colorsText = {
  light: 'text-brownish-700 dark:text-brownish-400',
  dark: 'text-brownish-700 dark:text-brownish-400',
  contrast: 'dark:text-white',
  success: 'text-greeny-500',
  danger: 'text-redish-500',
  warning: 'text-orangy-500',
  info: 'text-redish-500',
  empty: 'text-black dark:text-white'
}

export const colorsOutline = {
  light: [colorsText.light, 'border-brownish-100'],
  dark: [colorsText.dark, 'border-brownish-400'],
  contrast: [colorsText.contrast, 'border-brownish-900 dark:border-brownish-100'],
  success: [colorsText.success, 'border-greeny-500'],
  danger: [colorsText.danger, 'border-redish-500'],
  warning: [colorsText.warning, 'border-orangy-500'],
  info: [colorsText.info, 'border-redish-500'],
  empty: [colorsText.empty]
}
export const getButtonColor = (
  color: ColorType,
  isOutlined: boolean,
  hasHover: boolean,
  isActive = false
): string[] => {
  const colors = {
    ring: {
      light: 'ring-brownish-200 dark:ring-brownish-500 ',
      dark: 'ring-brownish-200 dark:ring-brownish-500 ',
      contrast: 'ring-brownish-300 dark:ring-brownish-400 ',
      success: 'ring-greeny-300 dark:ring-greeny-700 ',
      danger: 'ring-redish-300 dark:ring-redish-700 ',
      warning: 'ring-orangy-300 dark:ring-orangy-700 ',
      info: 'ring-blueish-300 dark:ring-blueish-700 ',
      empty: 'ring-brownish-200 dark:ring-brownish-500'
    },
    active: {
      light: 'bg-brownish-200 dark:bg-brownish-700 ',
      dark: 'bg-brownish-300 dark:bg-brownish-800 ',
      contrast: 'bg-brownish-700 dark:bg-brownish-100 ',
      success: 'bg-greeny-700 dark:bg-greeny-600 ',
      danger: 'bg-redish-700 dark:bg-redish-600 ',
      warning: 'bg-orangy-700 dark:bg-orangy-600 ',
      info: 'bg-blueish-700 dark:bg-blueish-600 ',
      empty: 'bg-transparent'
    },
    bg: {
      light: 'bg-brownish-100 text-black dark:bg-brownish-800 dark:text-white ',
      dark: 'bg-brownish-100 text-black dark:bg-brownish-800 dark:text-white ',
      contrast: 'bg-brownish-800 text-white dark:bg-brownish-100 dark:text-black ',
      success: 'bg-greeny-600 dark:bg-greeny-500 text-white ',
      danger: 'bg-redish-600 dark:bg-redish-500 text-white ',
      warning: 'bg-orangy-600 dark:bg-orangy-500 text-white ',
      info: 'bg-blueish-600 dark:bg-blueish-500 text-white ',
      empty: 'bg-transparent text-black dark:text-white'
    },
    bgHover: {
      light: 'hover:bg-brownish-200 hover:dark:bg-brownish-700 ',
      dark: 'hover:bg-brownish-200 hover:dark:bg-brownish-700 ',
      contrast: 'hover:bg-brownish-700 hover:dark:bg-brownish-100 ',
      success:
        'hover:bg-greeny-700 hover:border-greeny-700 hover:dark:bg-greeny-600 hover:dark:border-greeny-600 ',
      danger:
        'hover:bg-redish-700 hover:border-redish-700 hover:dark:bg-redish-600 hover:dark:border-redish-600 ',
      warning:
        'hover:bg-orangy-700 hover:border-orangy-700 hover:dark:bg-orangy-600 hover:dark:border-orangy-600 ',
      info: 'hover:bg-blueish-700 hover:border-blueish-700 hover:dark:bg-blueish-600 hover:dark:border-blueish-600 ',
      empty: 'hover:bg-transparent'
    },
    borders: {
      light: 'border-brownish-100 dark:border-brownish-800 ',
      dark: 'border-brownish-100 dark:border-brownish-800 ',
      contrast: 'border-brownish-800 dark:border-white ',
      success: 'border-greeny-600 dark:border-greeny-500 ',
      danger: 'border-redish-600 dark:border-redish-500 ',
      warning: 'border-orangy-600 dark:border-orangy-500 ',
      info: 'border-blueish-600 dark:border-blueish-500 ',
      empty: 'border-transparent'
    },
    text: {
      light: 'text-brownish-100 dark:text-brownish-800 ',
      dark: 'text-brownish-100 dark:text-brownish-800 ',
      contrast: 'dark:text-brownish-100 ',
      success: 'text-greeny-600 dark:text-greeny-500 ',
      danger: 'text-redish-600 dark:text-redish-500 ',
      warning: 'text-orangy-600 dark:text-orangy-500 ',
      info: 'text-blueish-600 dark:text-blueish-500 ',
      empty: 'text-black dark:text-white'
    },
    outlineHover: {
      light: 'hover:text-brownish-100 hover:dark:text-brownish-800 ',
      dark: 'hover:text-brownish-100 hover:dark:text-brownish-800 ',
      contrast:
        'hover:bg-brownish-800 hover:text-brownish-100 hover:dark:bg-brownish-100 hover:dark:text-black ',
      success:
        'hover:bg-greeny-600 hover:text-white hover:text-white hover:dark:text-white hover:dark:border-greeny-600 ',
      danger:
        'hover:bg-redish-600 hover:text-white hover:text-white hover:dark:text-white hover:dark:border-redish-600 ',
      warning:
        'hover:bg-orangy-600 hover:text-white hover:text-white hover:dark:text-white hover:dark:border-orangy-600 ',
      info: 'hover:bg-blueish-600 hover:text-white hover:dark:text-white hover:dark:border-blueish-600 ',
      empty: 'hover:text-black hover:dark:text-white '
    }
  }

  if (!colors.bg[color]) {
    return [color]
  }

  const isOutlinedProcessed = isOutlined && ['light', 'dark'].indexOf(color) < 0

  const base = [colors.borders[color], colors.ring[color]]

  if (isActive) {
    base.push(colors.active[color])
  } else {
    base.push(isOutlinedProcessed ? colors.text[color] : colors.bg[color])
  }

  if (hasHover) {
    base.push(isOutlinedProcessed ? colors.outlineHover[color] : colors.bgHover[color])
  }

  return base
}
