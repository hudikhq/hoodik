export const gradientBgBase = 'bg-gradient-to-tr'
export const gradientBgPurplePink = `${gradientBgBase} from-greeny-400 via-orangy-500 to-redish-500`
export const gradientBgDark = `${gradientBgBase} from-brownish-700 via-brownish-900 to-brownish-800`
export const gradientBgPinkRed = `${gradientBgBase} from-redish-400 via-redish-500 to-orangy-500`
export type ColorType =
  | 'white'
  | 'whiteDark'
  | 'lightDark'
  | 'light'
  | 'contrast'
  | 'success'
  | 'danger'
  | 'warning'
  | 'info'
  | 'empty'

export const colorsBgLight = {
  white: 'bg-brownish-100 text-black',
  whiteDark: 'bg-brownish-100 text-black',
  light: 'bg-brownish-100 text-black dark:bg-brownish-900/70 dark:text-white',
  lightDark: 'bg-brownish-100 text-black dark:bg-brownish-900/70 dark:text-white',
  contrast: 'bg-brownish-800 text-white dark:bg-brownish-100 dark:text-black',
  success: 'bg-greenish-500 border-greenish-500 text-white',
  danger: 'bg-redish-500 border-redish-500 text-white',
  warning: 'bg-orangy-500 border-orangy-500 text-white',
  info: 'bg-redish-500 border-redish-500 text-white',
  empty: 'bg-transparent text-black dark:text-white'
}

export const colorsText = {
  white: 'text-black dark:text-brownish-100',
  whiteDark: 'text-black dark:text-brownish-100',
  light: 'text-brownish-700 dark:text-brownish-400',
  lightDark: 'text-brownish-700 dark:text-brownish-400',
  contrast: 'dark:text-white',
  success: 'text-greenish-500',
  danger: 'text-redish-500',
  warning: 'text-orangy-500',
  info: 'text-redish-500',
  empty: 'text-black dark:text-white'
}

export const colorsOutline = {
  white: [colorsText.white, 'border-brownish-50'],
  whiteDark: [colorsText.white, 'border-brownish-700'],
  light: [colorsText.light, 'border-brownish-100'],
  lightDark: [colorsText.light, 'border-brownish-400'],
  contrast: [colorsText.contrast, 'border-brownish-900 dark:border-brownish-100'],
  success: [colorsText.success, 'border-greenish-500'],
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
      white: 'ring-brownish-200 dark:ring-brownish-500 ',
      whiteDark: 'ring-brownish-200 dark:ring-brownish-500 ',
      light: 'ring-brownish-200 dark:ring-brownish-500 ',
      lightDark: 'ring-brownish-200 dark:ring-brownish-500 ',
      contrast: 'ring-brownish-300 dark:ring-brownish-400 ',
      success: 'ring-greenish-300 dark:ring-greenish-700 ',
      danger: 'ring-redish-300 dark:ring-redish-700 ',
      warning: 'ring-orangy-300 dark:ring-orangy-700 ',
      info: 'ring-blueish-300 dark:ring-blueish-700 ',
      empty: 'ring-brownish-200 dark:ring-brownish-500'
    },
    active: {
      white: 'bg-brownish-100 ',
      whiteDark: 'bg-brownish-100 dark:bg-brownish-800 ',
      light: 'bg-brownish-200 dark:bg-brownish-700 ',
      lightDark: 'bg-brownish-200 dark:bg-brownish-700 ',
      contrast: 'bg-brownish-700 dark:bg-brownish-100 ',
      success: 'bg-greenish-700 dark:bg-greenish-600 ',
      danger: 'bg-redish-700 dark:bg-redish-600 ',
      warning: 'bg-orangy-700 dark:bg-orangy-600 ',
      info: 'bg-blueish-700 dark:bg-blueish-600 ',
      empty: 'bg-transparent'
    },
    bg: {
      white: 'bg-brownish-100 text-black ',
      whiteDark: 'bg-brownish-100 text-black dark:bg-brownish-900 dark:text-white ',
      light: 'bg-brownish-100 text-black dark:bg-brownish-800 dark:text-white ',
      lightDark: 'bg-brownish-100 text-black dark:bg-brownish-800 dark:text-white ',
      contrast: 'bg-brownish-800 text-white dark:bg-brownish-100 dark:text-black ',
      success: 'bg-greenish-600 dark:bg-greenish-500 text-white ',
      danger: 'bg-redish-600 dark:bg-redish-500 text-white ',
      warning: 'bg-orangy-600 dark:bg-orangy-500 text-white ',
      info: 'bg-blueish-600 dark:bg-blueish-500 text-white ',
      empty: 'bg-transparent text-black dark:text-white'
    },
    bgHover: {
      white: 'hover:bg-brownish-100 ',
      whiteDark: 'hover:bg-brownish-100 hover:dark:bg-brownish-800 ',
      light: 'hover:bg-brownish-200 hover:dark:bg-brownish-700 ',
      lightDark: 'hover:bg-brownish-200 hover:dark:bg-brownish-700 ',
      contrast: 'hover:bg-brownish-700 hover:dark:bg-brownish-100 ',
      success:
        'hover:bg-greenish-700 hover:border-greenish-700 hover:dark:bg-greenish-600 hover:dark:border-greenish-600 ',
      danger:
        'hover:bg-redish-700 hover:border-redish-700 hover:dark:bg-redish-600 hover:dark:border-redish-600 ',
      warning:
        'hover:bg-orangy-700 hover:border-orangy-700 hover:dark:bg-orangy-600 hover:dark:border-orangy-600 ',
      info: 'hover:bg-blueish-700 hover:border-blueish-700 hover:dark:bg-blueish-600 hover:dark:border-blueish-600 ',
      empty: 'hover:bg-transparent'
    },
    borders: {
      white: 'border-white ',
      whiteDark: 'border-white dark:border-brownish-900 ',
      light: 'border-brownish-100 dark:border-brownish-800 ',
      lightDark: 'border-brownish-100 dark:border-brownish-800 ',
      contrast: 'border-brownish-800 dark:border-white ',
      success: 'border-greenish-600 dark:border-greenish-500 ',
      danger: 'border-redish-600 dark:border-redish-500 ',
      warning: 'border-orangy-600 dark:border-orangy-500 ',
      info: 'border-blueish-600 dark:border-blueish-500 ',
      empty: 'border-transparent'
    },
    text: {
      white: 'text-white ',
      whiteDark: 'text-white dark:text-brownish-900 ',
      light: 'text-brownish-100 dark:text-brownish-800 ',
      lightDark: 'text-brownish-100 dark:text-brownish-800 ',
      contrast: 'dark:text-brownish-100 ',
      success: 'text-greenish-600 dark:text-greenish-500 ',
      danger: 'text-redish-600 dark:text-redish-500 ',
      warning: 'text-orangy-600 dark:text-orangy-500 ',
      info: 'text-blueish-600 dark:text-blueish-500 ',
      empty: 'text-black dark:text-white'
    },
    outlineHover: {
      white: 'hover:text-white ',
      whiteDark: 'hover:text-white hover:dark:text-brownish-900 ',
      light: 'hover:text-brownish-100 hover:dark:text-brownish-800 ',
      lightDark: 'hover:text-brownish-100 hover:dark:text-brownish-800 ',
      contrast:
        'hover:bg-brownish-800 hover:text-brownish-100 hover:dark:bg-brownish-100 hover:dark:text-black ',
      success:
        'hover:bg-greenish-600 hover:text-white hover:text-white hover:dark:text-white hover:dark:border-greenish-600 ',
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

  const isOutlinedProcessed = isOutlined && ['white', 'whiteDark', 'lightDark'].indexOf(color) < 0

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
