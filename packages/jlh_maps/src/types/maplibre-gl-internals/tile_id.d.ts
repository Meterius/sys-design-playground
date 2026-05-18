import type { OverscaledTileID as MapLibreOverscaledTileID } from 'maplibre-gl'

export interface CanonicalTileID {
  z: number
  x: number
  y: number
  key?: string
  toString?: () => string
}

export type OverscaledTileID = MapLibreOverscaledTileID & {
  canonical: CanonicalTileID
}
