import { i18n } from '@/i18n'

const SUBTYPE_CATEGORIES: Record<string, string> = {
  pdf: 'pdf',
  zip: 'archive',
  'x-tar': 'archive',
  gzip: 'archive',
  'x-7z-compressed': 'archive',
  'x-rar-compressed': 'archive',
  'vnd.rar': 'archive',
  msword: 'document',
  'vnd.openxmlformats-officedocument.wordprocessingml.document': 'document',
  'vnd.oasis.opendocument.text': 'document',
  rtf: 'document',
  'vnd.ms-excel': 'spreadsheet',
  'vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'spreadsheet',
  'vnd.oasis.opendocument.spreadsheet': 'spreadsheet',
  'vnd.ms-powerpoint': 'presentation',
  'vnd.openxmlformats-officedocument.presentationml.presentation': 'presentation',
  'vnd.oasis.opendocument.presentation': 'presentation'
}

/**
 * Human-readable label for a stored mime type: "image/jpeg" reads as
 * "Image", "dir" as "Folder". Unrecognized types fall back to the raw
 * mime string so nothing is ever hidden behind a wrong guess.
 */
export function prettyMime(mime: string | undefined | null): string {
  if (!mime) return ''

  const t = i18n.global.t
  if (mime === 'dir') return t('files.type.folder')

  const [type, subtype = ''] = mime.split('/')

  switch (type) {
    case 'image':
      return t('files.type.image')
    case 'video':
      return t('files.type.video')
    case 'audio':
      return t('files.type.audio')
    case 'font':
      return t('files.type.font')
    case 'text':
      return t('files.type.text')
  }

  const category = SUBTYPE_CATEGORIES[subtype]
  if (category) return t(`files.type.${category}`)

  return mime
}
