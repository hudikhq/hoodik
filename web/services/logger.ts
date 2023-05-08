/**
 * Regular logging function passed to the logger service
 */
function log(...args: any[]) {
  console.log(...args)
}

/**
 * Regular logging function passed to the logger service
 */
function info(...args: any[]) {
  console.info(...args)
}

/**
 * Regular logging function passed to the logger service
 */
function warn(...args: any[]) {
  console.warn(...args)
}

/**
 * Regular logging function passed to the logger service
 */
function error(...args: any[]) {
  console.error(...args)
}

/**
 * Regular logging function passed to the logger service
 */
function debug(...args: any[]) {
  if (import.meta.env.DEV) {
    console.log(...args)
  }
}

/**
 * Some ascii art for the logger when in production
 */
export function greeting() {
  console.log(
    '46_____ _______ _______ ____58_ _82__ 78_____\n' +
      '75_____ _______ _______ ____63_ _77__ 48__73_\n' +
      '79_____ _3524__ _7862__ _59795_ _____ 32_54__\n' +
      '87957__ 85__38_ 42__43_ 33__29_ _26__ 3649___\n' +
      '92__78_ 28__89_ 57__57_ 63__48_ _74__ 52_92__\n' +
      '65__49_ _9384__ _5572__ _6872__ 8629_ 85__26_\n' +
      '          end to end encrypted drive         '
  )
}

export { log, warn, error, info, debug }
