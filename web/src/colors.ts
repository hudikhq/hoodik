// The full-screen backdrop behind auth and link-entry cards. Named for the
// hues they actually run through — the old purplePink/pinkRed labels described
// neither, and the purple one had no caller.
const gradientBase = 'bg-gradient-to-tr'
export const gradientDark = `${gradientBase} from-brownish-700 via-brownish-900 to-brownish-800`
export const gradientCrimsonEmber = `${gradientBase} from-redish-400 via-redish-500 to-orangy-500`
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
  light: 'bg-paper-100 text-brownish-700 dark:bg-brownish-900/70 dark:text-white',
  dark: 'bg-paper-100 text-brownish-700 dark:bg-brownish-900/70 dark:text-white',
  contrast: 'bg-brownish-800 text-white dark:bg-brownish-100 dark:text-brownish-950',
  success: 'bg-greeny-500 border-greeny-500 text-white',
  danger: 'bg-redish-500 border-redish-500 text-white',
  warning: 'bg-orangy-500 border-orangy-500 text-white',
  info: 'bg-redish-500 border-redish-500 text-white',
  empty: 'bg-transparent text-brownish-700 dark:text-white'
}

// Outline and tinted-text modes put these on a charcoal surface, so each hue
// takes the step that clears AA there — the fill steps are far too dark as text.
export const colorsText = {
  light: 'text-brownish-700 dark:text-brownish-50',
  dark: 'text-brownish-700 dark:text-brownish-50',
  contrast: 'dark:text-white',
  success: 'text-greeny-500 dark:text-greeny-300',
  danger: 'text-redish-700 dark:text-redish-100',
  warning: 'text-orangy-800 dark:text-orangy-400',
  info: 'text-redish-700 dark:text-redish-100',
  empty: 'text-brownish-700 dark:text-white'
}

export const colorsOutline = {
  light: [colorsText.light, 'border-paper-300'],
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
      light: 'ring-paper-400 dark:ring-brownish-500 ',
      dark: 'ring-paper-400 dark:ring-brownish-500 ',
      contrast: 'ring-brownish-300 dark:ring-brownish-400 ',
      success: 'ring-greeny-300 dark:ring-greeny-700 ',
      danger: 'ring-redish-300 dark:ring-redish-700 ',
      warning: 'ring-orangy-300 dark:ring-orangy-700 ',
      info: 'ring-redish-300 dark:ring-redish-700 ',
      empty: 'ring-brownish-200 dark:ring-brownish-500'
    },
    active: {
      light: 'bg-paper-200 dark:bg-brownish-700 ',
      dark: 'bg-paper-200 dark:bg-brownish-800 ',
      contrast: 'bg-brownish-700 dark:bg-brownish-100 ',
      success: 'bg-greeny-700 dark:bg-greeny-600 ',
      danger: 'bg-redish-700 dark:bg-redish-600 ',
      warning: 'bg-orangy-700 dark:bg-orangy-600 ',
      info: 'bg-redish-600 dark:bg-redish-500 ',
      empty: 'bg-transparent'
    },
    bg: {
      light: 'bg-white text-brownish-700 dark:bg-brownish-800 dark:text-white ',
      dark: 'bg-white text-brownish-700 dark:bg-brownish-800 dark:text-white ',
      contrast: 'bg-brownish-800 text-white dark:bg-brownish-100 dark:text-brownish-950 ',
      success: 'bg-greeny-600 dark:bg-greeny-500 text-white ',
      danger: 'bg-redish-600 dark:bg-redish-500 text-white ',
      warning: 'bg-orangy-600 dark:bg-orangy-500 text-white ',
      info: 'bg-redish-500 dark:bg-redish-400 text-white ',
      empty: 'bg-transparent text-brownish-700 dark:text-white'
    },
    bgHover: {
      light: 'hover:bg-paper-100 hover:dark:bg-brownish-700 ',
      dark: 'hover:bg-paper-100 hover:dark:bg-brownish-700 ',
      contrast: 'hover:bg-brownish-700 hover:dark:bg-brownish-100 ',
      success:
        'hover:bg-greeny-700 hover:border-greeny-700 hover:dark:bg-greeny-600 hover:dark:border-greeny-600 ',
      danger:
        'hover:bg-redish-700 hover:border-redish-700 hover:dark:bg-redish-600 hover:dark:border-redish-600 ',
      warning:
        'hover:bg-orangy-700 hover:border-orangy-700 hover:dark:bg-orangy-600 hover:dark:border-orangy-600 ',
      info: 'hover:bg-redish-600 hover:border-redish-600 hover:dark:bg-redish-300 hover:dark:border-redish-300 ',
      empty: 'hover:bg-transparent'
    },
    borders: {
      light: 'border-paper-300 dark:border-brownish-500 ',
      dark: 'border-paper-300 dark:border-brownish-500 ',
      contrast: 'border-brownish-800 dark:border-white ',
      success: 'border-greeny-600 dark:border-greeny-500 ',
      danger: 'border-redish-600 dark:border-redish-500 ',
      warning: 'border-orangy-600 dark:border-orangy-500 ',
      info: 'border-redish-500 dark:border-redish-400 ',
      empty: 'border-transparent'
    },
    // Outline buttons draw this hue as text on a charcoal panel, so every step
    // here is the AA-clearing text step rather than the fill step.
    text: {
      light: 'text-brownish-700 dark:text-brownish-50 ',
      dark: 'text-brownish-700 dark:text-brownish-50 ',
      contrast: 'dark:text-brownish-50 ',
      success: 'text-greeny-500 dark:text-greeny-300 ',
      danger: 'text-redish-700 dark:text-redish-100 ',
      warning: 'text-orangy-800 dark:text-orangy-400 ',
      info: 'text-redish-700 dark:text-redish-100 ',
      empty: 'text-brownish-700 dark:text-white'
    },
    outlineHover: {
      light: 'hover:text-brownish-100 hover:dark:text-brownish-800 ',
      dark: 'hover:text-brownish-100 hover:dark:text-brownish-800 ',
      contrast:
        'hover:bg-brownish-800 hover:text-brownish-100 hover:dark:bg-brownish-100 hover:dark:text-brownish-950 ',
      success:
        'hover:bg-greeny-600 hover:text-white hover:text-white hover:dark:text-white hover:dark:border-greeny-600 ',
      danger:
        'hover:bg-redish-600 hover:text-white hover:text-white hover:dark:text-white hover:dark:border-redish-600 ',
      warning:
        'hover:bg-orangy-600 hover:text-white hover:text-white hover:dark:text-white hover:dark:border-orangy-600 ',
      info: 'hover:bg-redish-500 hover:text-white hover:dark:text-white hover:dark:border-redish-500 ',
      empty: 'hover:text-brownish-700 hover:dark:text-white '
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
