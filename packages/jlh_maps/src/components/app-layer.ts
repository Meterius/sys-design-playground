import {
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  type GeoJSONFeature,
  type Map as MapLibreMap,
} from 'maplibre-gl'
import { sync_tiles, sync_view } from 'jlh_maps_app'

type TileKey = string

interface TileCoord {
  z: number
  x: number
  y: number
}

export class AppLayer implements CustomLayerInterface {
  id = 'app-layer'
  type = 'custom' as const
  renderingMode = '3d' as const

  private map!: MapLibreMap

  constructor(private readonly canvasSelector: string) {
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map
  }

  render(_gl: WebGLRenderingContext | WebGL2RenderingContext, options: CustomRenderMethodInput): void {
    const center = this.map.getCenter()
    const canvas = this.map.getCanvas()

    sync_view(
      this.canvasSelector,
      canvas.width,
      canvas.height,
      this.map.getZoom(),
      this.map.getPitch(),
      this.map.getBearing(),
      center.lng,
      center.lat,
      JSON.stringify(Array.from(options.defaultProjectionData.mainMatrix)),
    )

    sync_tiles(this.canvasSelector, this.getVisibleTileKeys().join(';'))
  }

  onRemove(): void {
  }

  private getVisibleTileKeys() {
    const tileKeys = new Set<TileKey>()

    try {
      const features = this.map.querySourceFeatures('openmaptiles')

      for (const feature of features) {
        const coord = this.getTileCoord(feature)
        if (!coord) continue

        tileKeys.add(`${coord.z}/${coord.x}/${coord.y}`)
      }
    } catch {
      return this.getFallbackVisibleTileKeys()
    }

    return tileKeys.size > 0 ? [...tileKeys] : this.getFallbackVisibleTileKeys()
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

  private getFallbackVisibleTileKeys() {
    const zoom = Math.max(0, Math.floor(this.map.getZoom()))
    const center = this.map.getCenter()
    const centerTile = this.lngLatToTile(center.lng, center.lat, zoom)
    const tileCount = 2 ** zoom
    const keys: TileKey[] = []

    for (let dx = -2; dx <= 2; dx++) {
      for (let dy = -2; dy <= 2; dy++) {
        const x = ((centerTile.x + dx) % tileCount + tileCount) % tileCount
        const y = centerTile.y + dy

        if (y < 0 || y >= tileCount) continue

        keys.push(`${zoom}/${x}/${y}`)
      }
    }

    return keys
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
}
