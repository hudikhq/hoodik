/* eslint-env node */

const redish = {
  50: '#F58596',
  100: '#E2677B',
  200: '#D8566B',
  300: '#BA4054',
  400: '#A63446', // native
  500: '#A43144',
  600: '#96293B',
  700: '#811C2C',
  800: '#610F1C',
  900: '#300C13',
  950: '#1C0508'
}

module.exports = {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    colors: {
      transparent: 'transparent',
      white: '#FFFFFF',
      'dirty-white': '#EEEEEE',
      // Light-mode neutrals. The brownish ramp starts at #898989, which is
      // why early light surfaces came out mid-gray; paper carries the light
      // end of the scale instead.
      paper: {
        50: '#FAFAF9',
        100: '#F1F1EF',
        200: '#E5E5E2',
        300: '#D2D2CE',
        400: '#B9B9B4'
      },
      redish,
      primary: redish,
      brownish: {
        // 50 is the muted-text step. It sits at #939393 rather than the
        // older #898989, which measured 4.49:1 against the #232323 panel —
        // a hair under AA for the small text it carries.
        50: '#939393',
        100: '#787878',
        200: '#666666',
        300: '#555555',
        400: '#4a4a4a',
        500: '#393939',
        600: '#303030',
        700: '#1E1E1E', // native
        800: '#232323',
        900: '#181818',
        950: '#0A0908'
      },
      orangy: {
        50: '#FAD4B8',
        100: '#F4C9A9',
        200: '#F5C49F',
        300: '#F2B88C',
        400: '#F2AC78',
        500: '#EE9B5C',
        600: '#EE8434', // native
        700: '#C76F2C',
        800: '#9F5822',
        900: '#48250C',
        950: '#2E1706'
      },
      greeny: {
        50: '#D8F1BE',
        100: '#D1F0B1',
        200: '#C1E1A1',
        300: '#94BC6A',
        400: '#658D3D',
        500: '#4E7228',
        600: '#2F500E',
        700: '#2E500A',
        800: '#2D5207',
        900: '#223E05',
        950: '#182E02'
      },
      blueish: {
        50: '#B8CCEB',
        100: '#A9C0E7',
        200: '#8BA9E0',
        300: '#6B8ED6',
        400: '#4D6EC9',
        500: '#586994', // native
        600: '#4A5A7A',
        700: '#3C4C60',
        800: '#2E3E46',
        900: '#20202C',
        950: '#0F0F15'
      },
    },
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif']
      },
      zIndex: {
        '-1': '-1'
      },
      flexGrow: {
        5: '5'
      },
      maxHeight: {
        'screen-menu': 'calc(100vh - 3.5rem)',
        modal: 'calc(100vh - 160px)'
      },
      transitionProperty: {
        position: 'right, left, top, bottom, margin, padding',
        textColor: 'color'
      },
      keyframes: {
        'fade-out': {
          from: { opacity: 1 },
          to: { opacity: 0 }
        },
        'fade-in': {
          from: { opacity: 0 },
          to: { opacity: 1 }
        }
      },
      animation: {
        'fade-out': 'fade-out 250ms ease-in-out',
        'fade-in': 'fade-in 250ms ease-in-out'
      }
    }
  },
  plugins: [require('@tailwindcss/forms')]
}
