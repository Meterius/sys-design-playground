import type { LngLatBounds } from 'maplibre-gl'
import type { CanonicalTileID } from './tile_id'

export function tileIdToLngLatBounds(tileId: CanonicalTileID): LngLatBounds
