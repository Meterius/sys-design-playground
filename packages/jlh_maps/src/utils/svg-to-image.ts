export type SvgImageBoundingBox = {
  x: number
  y: number
  width: number
  height: number
}

export type SvgToImageOptions = {
  width: number
  height?: number
  pixelRatio?: number
  color: string
}

export type SvgToImageResult = {
  image: ImageData
}

const SVG_NAMESPACE = 'http://www.w3.org/2000/svg'
const SVG_PRESENTATION_ATTRIBUTES = [
  'clip-rule',
  'fill',
  'fill-rule',
  'stroke',
  'stroke-linecap',
  'stroke-linejoin',
  'stroke-miterlimit',
  'stroke-width',
  'style',
] as const

const parseSvg = (svgSource: string) => {
  const svg = new DOMParser().parseFromString(svgSource, 'image/svg+xml').documentElement

  if (svg.tagName.toLowerCase() !== 'svg') {
    throw new Error('Expected SVG source to parse into an <svg> element')
  }

  return svg as unknown as SVGSVGElement
}

const parseViewBox = (svg: SVGSVGElement) => {
  const values = svg
    .getAttribute('viewBox')
    ?.trim()
    .split(/\s+/)
    .map((value) => Number(value))

  if (values?.length === 4 && values.every((value) => Number.isFinite(value))) {
    const [x, y, width, height] = values as [number, number, number, number]
    return `${x} ${y} ${width} ${height}`
  }

  return `0 0 ${Number(svg.getAttribute('width') ?? 16)} ${Number(svg.getAttribute('height') ?? 16)}`
}

const escapeAttribute = (value: string) =>
  value
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')

const getSvgPresentationAttributes = (svg: SVGSVGElement) =>
  SVG_PRESENTATION_ATTRIBUTES.flatMap((name) => {
    const value = svg.getAttribute(name)
    if (value === null) return []

    return [`${name}="${escapeAttribute(value)}"`]
  }).join(' ')

const buildRenderableSvg = (
  svg: SVGSVGElement,
  { width, height, color }: Required<SvgToImageOptions>,
) =>
  `
<svg xmlns="${SVG_NAMESPACE}" width="${width}" height="${height}" viewBox="${parseViewBox(svg)}" color="${escapeAttribute(color)}">
  <g ${getSvgPresentationAttributes(svg)}>${svg.innerHTML}</g>
</svg>`.trim()

const loadSvgImage = async (svgSource: string) => {
  const url = URL.createObjectURL(new Blob([svgSource], { type: 'image/svg+xml' }))
  const image = new Image()

  image.src = url
  await image.decode()

  return {
    image,
    revoke: () => URL.revokeObjectURL(url),
  }
}

export const svgToImage = async (
  svgSource: string,
  { width, height = width, pixelRatio = 1, color }: SvgToImageOptions,
): Promise<SvgToImageResult> => {
  const loadedImage = await loadSvgImage(
    buildRenderableSvg(parseSvg(svgSource), { width, height, pixelRatio, color }),
  )
  const canvas = document.createElement('canvas')
  canvas.width = Math.ceil(width * pixelRatio)
  canvas.height = Math.ceil(height * pixelRatio)

  const ctx = canvas.getContext('2d')
  if (!ctx) {
    loadedImage.revoke()
    const emptyImage = new ImageData(canvas.width, canvas.height)

    return {
      image: emptyImage,
    }
  }

  try {
    ctx.drawImage(loadedImage.image, 0, 0, canvas.width, canvas.height)
  } finally {
    loadedImage.revoke()
  }

  const image = ctx.getImageData(0, 0, canvas.width, canvas.height)

  return {
    image,
  }
}
