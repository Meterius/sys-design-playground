import type { Terrain } from './terrain'
import type { OverscaledTileID } from './tile_id'

export interface Tile {
  tileID?: OverscaledTileID | { key?: string; toString?: () => string }
  rtt?: unknown[] | null
  rttFingerprint?: Record<string, string | number | boolean | null | undefined> | null
  dem?: unknown
}

export interface Map {
  transform: {
    zoom: number
    getProjectionDataForCustomLayer?: () => { mainMatrix?: ArrayLike<number> }
    projectionData?: { mainMatrix?: ArrayLike<number> }
    modelViewProjectionMatrix?: ArrayLike<number>
  }
  terrain: Terrain | null
  on(type: string, callback: () => void): unknown
  off(type: string, callback: () => void): unknown
  getCenter(): { lng: number; lat: number }
  getCanvas(): HTMLCanvasElement
  getPitch(): number
  getBearing(): number
  querySourceFeatures(
    sourceId: string,
    options?: { sourceLayer?: string },
  ): Array<{
    id?: unknown
    geometry?: import('geojson').Geometry
    properties?: Record<string, unknown> | null
    _z?: number
    _x?: number
    _y?: number
    _vectorTileFeature?: { id?: unknown }
    toJSON(): {
      geometry?: import('geojson').Geometry | null
      properties?: Record<string, unknown> | null
    }
  }>
  coveringTiles(options: {
    tileSize: number
  }): Array<{ canonical: import('./tile_id').CanonicalTileID }>
}
