import {
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  type GeoJSONFeature,
  type Map as MapLibreMap,
} from 'maplibre-gl'
import { tileIdToLngLatBounds } from 'maplibre-gl/src/tile/tile_id_to_lng_lat_bounds.ts'
import { sync_tile_texture, sync_tiles, sync_view } from 'jlh_maps_app'

type TileKey = string
type GL = WebGLRenderingContext | WebGL2RenderingContext

interface TileCoord {
  z: number
  x: number
  y: number
}

interface SyncedTile {
  key: TileCoord
  bounds_lnglat: [[number, number], [number, number]]
}

interface MapLibreTexture {
  texture?: WebGLTexture
  size?: [number, number]
  width?: number
  height?: number
}

interface MapLibreRttTile {
  tileID?: {
    canonical?: TileCoord
  }
  rtt?: Array<{
    id?: string | number
    stamp?: string | number
  }>
  rttFingerprint?: Record<string, string>
}

interface MapLibreTerrain {
  qualityFactor?: number
  tileManager?: {
    tileSize?: number
    getRenderableTiles?: () => MapLibreRttTile[]
  }
}

interface MapLibrePainter {
  renderToTexture?: {
    getTexture?: (tile: MapLibreRttTile) => MapLibreTexture | undefined
  }
  terrain?: MapLibreTerrain
}

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-expect-error
interface MapLibrePrivateMap extends MapLibreMap {
  painter?: MapLibrePainter
  terrain?: MapLibreTerrain
}

export class AppLayer implements CustomLayerInterface {
  id = 'app-layer'
  type = 'custom' as const
  renderingMode = '3d' as const

  private readonly maxTextureCopiesPerFrame = 4
  private readonly textureStamps = new Map<TileKey, string>()
  private map!: MapLibreMap
  private readFramebuffer: WebGLFramebuffer | null = null

  constructor(private readonly canvasSelector: string) {
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map
    this.readFramebuffer = gl.createFramebuffer()
  }

  render(gl: WebGLRenderingContext | WebGL2RenderingContext, options: CustomRenderMethodInput): void {
    const center = this.map.getCenter()
    const canvas = this.map.getCanvas()

    sync_view(
      this.canvasSelector,
      canvas.width,
      canvas.height,
      this.map.transform.zoom,
      this.map.getPitch(),
      this.map.getBearing(),
      center.lng,
      center.lat,
      JSON.stringify(Array.from(options.defaultProjectionData.mainMatrix)),
    )

    const visibleTiles = this.getVisibleTiles()
    sync_tiles(this.canvasSelector, JSON.stringify(visibleTiles))
    //this.syncTerrainRenderTextures(gl)
  }

  onRemove(_map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    if (this.readFramebuffer) {
      gl.deleteFramebuffer(this.readFramebuffer)
      this.readFramebuffer = null
    }
    this.textureStamps.clear()
  }

  private getVisibleTiles() {
    const tiles = new Map<TileKey, SyncedTile>()

    const tileManager = this.map.style.tileManagers['openmaptiles']!

    tileManager.getRenderableIds().map((id) => {
      const key = tileManager.getTileByID(id)!.tileID.canonical
      const bounds = tileIdToLngLatBounds(key)
      tiles.set(`${key.z}/${key.x}/${key.y}`, {
        key: { z: key.z, x: key.x, y: key.y },
        bounds_lnglat: [
          [bounds.getWest(), bounds.getSouth()],
          [bounds.getEast(), bounds.getNorth()],
        ],
      })
    })

    // (this.map as unknown as MapLibrePrivateMap).terrain?.tileManager?.getRenderableTiles?.().forEach((tile) => {
    //   const key = tile.tileID?.canonical
    //   if (!key) return
    //   const bounds = tileIdToLngLatBounds(key)
    //   tiles.set(`${key.z}/${key.x}/${key.y}`, {
    //     key: { z: key.z, x: key.x, y: key.y },
    //     bounds_lnglat: [
    //       [bounds.getWest(), bounds.getSouth()],
    //       [bounds.getEast(), bounds.getNorth()],
    //     ],
    //   })
    // })

    return [...tiles.values()]
  }

  private getTileCoord(feature: GeoJSONFeature): TileCoord | undefined {
    const maybeFeature = feature as GeoJSONFeature & {
      _z?: number
      _x?: number
      _y?: number
    }

    if (
      typeof maybeFeature._z !== 'number' ||
      typeof maybeFeature._x !== 'number' ||
      typeof maybeFeature._y !== 'number'
    ) {
      return undefined
    }

    return {
      z: maybeFeature._z,
      x: maybeFeature._x,
      y: maybeFeature._y,
    }
  }

  private lngLatToTile(lng: number, lat: number, zoom: number) {
    const scale = 2 ** zoom
    const clampedLat = Math.max(-85.051129, Math.min(85.051129, lat))
    const latRad = (clampedLat * Math.PI) / 180

    return {
      x: Math.floor(((lng + 180) / 360) * scale),
      y: Math.floor(
        ((1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2) * scale,
      ),
    }
  }

  private syncTerrainRenderTextures(gl: GL) {
    if (!this.readFramebuffer) return

    const privateMap = this.map as unknown as MapLibrePrivateMap
    const painter = privateMap.painter
    const terrain = privateMap.terrain ?? painter?.terrain
    const renderToTexture = painter?.renderToTexture
    const tiles = terrain?.tileManager?.getRenderableTiles?.() ?? []

    if (!renderToTexture?.getTexture || tiles.length === 0) return

    let copied = 0
    for (const tile of tiles) {
      if (copied >= this.maxTextureCopiesPerFrame) break

      const key = this.getRttTileKey(tile)
      if (!key) continue

      const contentStamp = this.getRttContentStamp(tile)
      if (!contentStamp || this.textureStamps.get(key) === contentStamp) continue

      console.log(key, tile, contentStamp);

      const texture = this.getRttTexture(renderToTexture, tile)
      const webGlTexture = texture?.texture
      if (!webGlTexture) continue

      const [width, height] = this.getRttTextureSize(texture, terrain)
      if (width <= 0 || height <= 0) continue

      const pixels = this.readTexturePixels(gl, webGlTexture, width, height)
      if (!pixels) continue

      sync_tile_texture(this.canvasSelector, key, width, height, pixels)
      this.textureStamps.set(key, contentStamp)
      copied += 1
    }

    const tileKeys = new Set(tiles.map((tile) => this.getRttTileKey(tile)));
    for (const key of this.textureStamps.keys()) {
      if (!tileKeys.has(key)) {
        this.textureStamps.delete(key);
      }
    }
  }

  private getRttTexture(
    renderToTexture: NonNullable<MapLibrePainter['renderToTexture']>,
    tile: MapLibreRttTile,
  ) {
    try {
      return renderToTexture.getTexture?.(tile)
    } catch {
      return undefined
    }
  }

  private getRttTileKey(tile: MapLibreRttTile): TileKey | undefined {
    const canonical = tile.tileID?.canonical
    if (!canonical) return undefined

    return `${canonical.z}/${canonical.x}/${canonical.y}`
  }

  private getRttContentStamp(tile: MapLibreRttTile) {
    if (!tile.rtt?.length) return undefined

    const fingerprints = tile.rttFingerprint ? Object.entries(tile.rttFingerprint) : []
    if (fingerprints.length === 0) return undefined

    return fingerprints
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([source, fingerprint]) => `${source}:${fingerprint}`)
      .join('|')
  }

  private getRttTextureSize(texture: MapLibreTexture, terrain?: MapLibreTerrain) {
    if (Array.isArray(texture.size) && texture.size.length >= 2) {
      return [texture.size[0], texture.size[1]] as const
    }

    if (typeof texture.width === 'number' && typeof texture.height === 'number') {
      return [texture.width, texture.height] as const
    }

    const tileSize = terrain?.tileManager?.tileSize
    const qualityFactor = terrain?.qualityFactor ?? 1
    const fallbackSize = tileSize ? Math.round(tileSize * qualityFactor) : 512

    return [fallbackSize, fallbackSize] as const
  }

  private readTexturePixels(gl: GL, texture: WebGLTexture, width: number, height: number) {
    const previousFramebuffer = gl.getParameter(gl.FRAMEBUFFER_BINDING) as WebGLFramebuffer | null
    const previousTexture = gl.getParameter(gl.TEXTURE_BINDING_2D) as WebGLTexture | null
    const previousPackAlignment = gl.getParameter(gl.PACK_ALIGNMENT) as number

    gl.bindFramebuffer(gl.FRAMEBUFFER, this.readFramebuffer)
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, texture, 0)

    const status = gl.checkFramebufferStatus(gl.FRAMEBUFFER)
    if (status !== gl.FRAMEBUFFER_COMPLETE) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, previousFramebuffer)
      gl.bindTexture(gl.TEXTURE_2D, previousTexture)
      return undefined
    }

    const pixels = new Uint8Array(width * height * 4)
    gl.pixelStorei(gl.PACK_ALIGNMENT, 1)
    gl.readPixels(0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels)
    gl.pixelStorei(gl.PACK_ALIGNMENT, previousPackAlignment)

    gl.bindFramebuffer(gl.FRAMEBUFFER, previousFramebuffer)
    gl.bindTexture(gl.TEXTURE_2D, previousTexture)

    return this.flipRgbaRows(pixels, width, height)
  }

  private flipRgbaRows(pixels: Uint8Array, width: number, height: number) {
    const rowSize = width * 4
    const flipped = new Uint8Array(pixels.length)

    for (let y = 0; y < height; y++) {
      const sourceOffset = y * rowSize
      const targetOffset = (height - y - 1) * rowSize
      flipped.set(pixels.subarray(sourceOffset, sourceOffset + rowSize), targetOffset)
    }

    return flipped
  }
}
