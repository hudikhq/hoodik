/**
 * Client-side PDF export from the Milkdown editor.
 * Uses html2pdf.js to render the editor content to a downloadable PDF.
 */
export async function exportPdf(editorWrapper: HTMLElement | undefined, fileName: string) {
  const html2pdf = (await import('html2pdf.js')).default

  const editorEl = editorWrapper?.querySelector('.ProseMirror')
  if (!editorEl) return

  const container = document.createElement('div')
  container.innerHTML = editorEl.innerHTML
  container.style.padding = '40px'
  container.style.maxWidth = '700px'
  container.style.margin = '0 auto'
  container.style.fontFamily = '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif'
  container.style.fontSize = '14px'
  container.style.lineHeight = '1.6'
  container.style.color = '#1a1a1a'

  container.querySelectorAll('pre').forEach((pre) => {
    pre.style.background = '#f5f5f5'
    pre.style.padding = '12px 16px'
    pre.style.borderRadius = '6px'
    pre.style.fontSize = '13px'
    pre.style.overflow = 'hidden'
  })
  container.querySelectorAll('code').forEach((code) => {
    if (code.parentElement?.tagName !== 'PRE') {
      code.style.background = '#f0f0f0'
      code.style.padding = '2px 5px'
      code.style.borderRadius = '3px'
      code.style.fontSize = '13px'
    }
    code.style.color = '#1a1a1a'
  })
  container.querySelectorAll('blockquote').forEach((bq) => {
    bq.style.borderLeft = '3px solid #ccc'
    bq.style.paddingLeft = '16px'
    bq.style.color = '#555'
  })
  container.querySelectorAll('a').forEach((a) => {
    a.style.color = '#0366d6'
  })
  container.querySelectorAll('th, td').forEach((cell) => {
    ;(cell as HTMLElement).style.border = '1px solid #ddd'
    ;(cell as HTMLElement).style.padding = '6px 12px'
  })
  container.querySelectorAll('hr').forEach((hr) => {
    hr.style.border = 'none'
    hr.style.borderTop = '1px solid #ddd'
  })

  // Convert images to inline base64 so html2canvas can render them
  const imgPromises = Array.from(container.querySelectorAll('img')).map(async (img) => {
    img.style.maxWidth = '100%'
    const src = img.src
    if (!src || src.startsWith('data:')) return

    try {
      const originalImg = editorEl.querySelector(`img[src="${img.getAttribute('src')}"]`) as HTMLImageElement | null
      const sourceImg = originalImg && originalImg.naturalWidth > 0 ? originalImg : null

      if (sourceImg) {
        const canvas = document.createElement('canvas')
        canvas.width = sourceImg.naturalWidth
        canvas.height = sourceImg.naturalHeight
        const ctx = canvas.getContext('2d')
        if (ctx) {
          ctx.drawImage(sourceImg, 0, 0)
          try {
            img.src = canvas.toDataURL('image/png')
            return
          } catch {
            // Canvas tainted by cross-origin — fall through to fetch
          }
        }
      }

      const response = await fetch(src, { credentials: 'include' })
      const blob = await response.blob()
      const dataUrl = await new Promise<string>((resolve) => {
        const reader = new FileReader()
        reader.onloadend = () => resolve(reader.result as string)
        reader.readAsDataURL(blob)
      })
      img.src = dataUrl
    } catch {
      img.remove()
    }
  })
  await Promise.all(imgPromises)

  container.removeAttribute('contenteditable')
  container.querySelectorAll('[contenteditable]').forEach((el) => {
    el.removeAttribute('contenteditable')
  })

  const name = (fileName || 'document').replace(/\.md$/i, '')

  await html2pdf()
    .set({
      margin: 10,
      filename: `${name}.pdf`,
      html2canvas: { scale: 2 },
      jsPDF: { unit: 'mm', format: 'a4', orientation: 'portrait' }
    })
    .from(container)
    .save()
}
