/* eslint-env node */

const plugin = require('tailwindcss/plugin')

module.exports = {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  darkMode: 'class', // or 'media' or 'class'
  theme: {
    colors: {
      transparent: 'transparent',
      white: '#FFFFFF',
      'dirty-white': '#EEEEEE',
      redish: {
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
      },
      brownish: {
        50: '#898989',
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
      asideScrollbars: {
        light: 'light',
        gray: 'brownish'
      },
      extend: {
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
    plugins: [
      require('@tailwindcss/forms'),
      plugin(function ({ matchUtilities, theme }) {
        matchUtilities(
          {
            'aside-scrollbars': (value) => {
              const track = value === 'light' ? '100' : '900'
              const thumb = value === 'light' ? '300' : '600'
              const color = value === 'light' ? 'gray' : value

              return {
                scrollbarWidth: 'thin',
                scrollbarColor: `${theme(`colors.${color}.${thumb}`)} ${theme(
                  `colors.${color}.${track}`
                )}`,
                '&::-webkit-scrollbar': {
                  width: '8px',
                  height: '8px'
                },
                '&::-webkit-scrollbar-track': {
                  backgroundColor: theme(`colors.${color}.${track}`)
                },
                '&::-webkit-scrollbar-thumb': {
                  borderRadius: '0.25rem',
                  backgroundColor: theme(`colors.${color}.${thumb}`)
                }
              }
            }
          },
          { values: theme('asideScrollbars') }
        )
      })
    ]
  }
}
