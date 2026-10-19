/**
 * Plain-text preview: which files qualify, and how their bytes become lines.
 *
 * Browsers report an empty `File.type` for most source and config files
 * (`.yaml`, `.rs`, `.toml`, `Dockerfile`, ...), so uploads land as
 * `application/octet-stream` and the mime alone can't be trusted. The name
 * is the fallback, and it also covers files uploaded before this existed.
 */

/** Displayed bytes are capped so a huge log can't freeze the tab. */
export const TEXT_PREVIEW_MAX_BYTES = 1024 * 1024
export const TEXT_PREVIEW_MAX_LINES = 50_000

const TEXT_MIMES = new Set([
  'application/json',
  'application/ld+json',
  'application/x-ndjson',
  'application/xml',
  'application/xhtml+xml',
  'application/yaml',
  'application/x-yaml',
  'application/toml',
  'application/x-toml',
  'application/javascript',
  'application/x-javascript',
  'application/ecmascript',
  'application/typescript',
  'application/x-typescript',
  'application/x-sh',
  'application/x-shellscript',
  'application/x-csh',
  'application/x-httpd-php',
  'application/x-perl',
  'application/x-python',
  'application/x-ruby',
  'application/sql',
  'application/graphql',
  'application/x-tex',
  'application/x-latex',
  'application/x-subrip',
  'application/x-wine-extension-ini',
  'application/x-desktop',
  'application/x-x509-ca-cert',
  'application/x-pem-file',
  'application/pgp-keys',
  'application/pgp-signature',
  'application/x-sql'
])

const TEXT_EXTENSIONS = new Set([
  // Plain text, logs, docs
  'txt', 'text', 'log', 'out', 'nfo', 'diz', 'rst', 'adoc', 'asciidoc', 'org', 'tex', 'bib',
  'srt', 'vtt', 'diff', 'patch',
  // Data and config
  'json', 'jsonc', 'json5', 'ndjson', 'jsonl', 'geojson', 'csv', 'tsv', 'psv', 'xml', 'xsd', 'xsl',
  'plist', 'yaml', 'yml', 'toml', 'ini', 'cfg', 'conf', 'config', 'properties', 'env', 'lock',
  'reg', 'desktop', 'service', 'socket', 'timer', 'tf', 'tfvars', 'hcl', 'nix', 'graphql', 'gql',
  'proto', 'sql', 'prisma', 'csproj', 'sln', 'gradle', 'cmake', 'mk', 'pem', 'crt', 'cer', 'csr',
  'asc', 'ics', 'vcf',
  // Web
  'html', 'htm', 'xhtml', 'css', 'scss', 'sass', 'less', 'styl', 'js', 'mjs', 'cjs', 'jsx', 'ts',
  'mts', 'cts', 'tsx', 'vue', 'svelte', 'astro', 'php', 'erb', 'ejs', 'hbs', 'mustache', 'twig',
  'liquid', 'pug', 'jade', 'wat',
  // Shell and scripting
  'sh', 'bash', 'zsh', 'fish', 'ksh', 'csh', 'ps1', 'psm1', 'psd1', 'bat', 'cmd', 'awk', 'sed',
  'py', 'pyi', 'pyw', 'rb', 'pl', 'pm', 'lua', 'r', 'jl', 'tcl', 'vim', 'el', 'lisp', 'cl',
  'clj', 'cljs', 'edn', 'scm', 'rkt',
  // Compiled languages
  'c', 'h', 'cc', 'cpp', 'cxx', 'hh', 'hpp', 'hxx', 'ino', 'cs', 'fs', 'fsx', 'vb', 'java',
  'kt', 'kts', 'scala', 'sc', 'groovy', 'go', 'rs', 'swift', 'm', 'mm', 'dart', 'zig', 'nim',
  'd', 'v', 'sv', 'vhd', 'vhdl', 'hs', 'lhs', 'ml', 'mli', 'elm', 'ex', 'exs', 'erl', 'hrl',
  'pas', 'f', 'f90', 'f95', 'asm', 's', 'cob', 'sol', 'gd', 'glsl', 'hlsl', 'wgsl', 'shader'
])

/** Extensionless or dot-files that are text by convention. */
const TEXT_FILENAMES = new Set([
  'makefile', 'gnumakefile', 'dockerfile', 'containerfile', 'vagrantfile', 'gemfile', 'rakefile',
  'procfile', 'justfile', 'jenkinsfile', 'cmakelists.txt', 'license', 'licence', 'copying',
  'readme', 'changelog', 'authors', 'contributors', 'notice', 'todo', 'codeowners',
  '.gitignore', '.gitattributes', '.gitmodules', '.gitconfig', '.dockerignore', '.npmignore',
  '.npmrc', '.nvmrc', '.yarnrc', '.editorconfig', '.prettierrc', '.eslintrc', '.babelrc',
  '.env', '.bashrc', '.bash_profile', '.zshrc', '.profile', '.vimrc', '.inputrc', '.htaccess',
  'known_hosts', 'authorized_keys', 'id_rsa.pub', 'id_ed25519.pub', 'hosts', 'fstab', 'crontab'
])

/**
 * Whether a file is worth opening in the plain-text viewer. Markdown is
 * deliberately not excluded here: callers rank the markdown viewer first.
 */
export function isTextFile(item: { name?: string; mime?: string }): boolean {
  const mime = (item.mime || '').toLowerCase().split(';')[0].trim()
  if (mime.startsWith('text/') || TEXT_MIMES.has(mime)) return true
  if (mime.startsWith('application/') && /\+(json|xml|yaml)$/.test(mime)) return true

  const name = (item.name || '').toLowerCase()
  if (!name) return false
  if (TEXT_FILENAMES.has(name)) return true
  // `.env.local`, `.env.production`, ...
  if (name.startsWith('.env.')) return true

  const dot = name.lastIndexOf('.')
  if (dot <= 0 || dot === name.length - 1) return false
  return TEXT_EXTENSIONS.has(name.slice(dot + 1))
}

export interface DecodedText {
  lines: string[]
  /** Something was cut off: by the byte cap, the line cap, or a partial download. */
  truncated: boolean
  /** NUL bytes in the sample, so this is most likely not text after all. */
  binary: boolean
}

const BINARY_SNIFF_BYTES = 8000

function detectEncoding(bytes: Uint8Array): { encoding: string; offset: number } {
  if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
    return { encoding: 'utf-8', offset: 3 }
  }
  if (bytes[0] === 0xff && bytes[1] === 0xfe) return { encoding: 'utf-16le', offset: 2 }
  if (bytes[0] === 0xfe && bytes[1] === 0xff) return { encoding: 'utf-16be', offset: 2 }
  return { encoding: 'utf-8', offset: 0 }
}

/**
 * Turn decrypted bytes into displayable lines.
 *
 * `partial` says the bytes are only the head of the file (e.g. its first
 * chunk), so the result is truncated even if it fits under the caps.
 */
export function decodeText(bytes: Uint8Array, partial = false): DecodedText {
  const { encoding, offset } = detectEncoding(bytes)

  // UTF-16 is full of NULs, so the sniff only applies to the 8-bit encodings.
  if (encoding === 'utf-8') {
    const sniffEnd = Math.min(bytes.length, offset + BINARY_SNIFF_BYTES)
    for (let i = offset; i < sniffEnd; i++) {
      if (bytes[i] === 0) return { lines: [], truncated: false, binary: true }
    }
  }

  let truncated = partial
  let end = bytes.length
  if (end - offset > TEXT_PREVIEW_MAX_BYTES) {
    end = offset + TEXT_PREVIEW_MAX_BYTES
    truncated = true
  }

  // `stream: true` holds back a multi-byte sequence split by the cut instead
  // of turning it into a replacement character.
  const text = new TextDecoder(encoding).decode(bytes.subarray(offset, end), {
    stream: truncated
  })

  let lines = text.split(/\r\n|\r|\n/)
  if (truncated) {
    // The last line was cut mid-way; showing half of it reads as corruption.
    if (lines.length > 1) lines.pop()
  } else if (lines.length > 1 && lines[lines.length - 1] === '') {
    lines.pop()
  }

  if (lines.length > TEXT_PREVIEW_MAX_LINES) {
    lines = lines.slice(0, TEXT_PREVIEW_MAX_LINES)
    truncated = true
  }

  return { lines, truncated, binary: false }
}
