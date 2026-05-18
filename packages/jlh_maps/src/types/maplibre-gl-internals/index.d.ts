import type {
  GeoJSONFeature,
  Map as MapLibreMap,
  QuerySourceFeatureOptions,
  Tile as MapLibreTile,
} from 'maplibre-gl'
import type { DEMData } from './dem_data'
import type { Terrain } from './terrain'
import type { CanonicalTileID, OverscaledTileID } from './tile_id'

export type { DEMData } from './dem_data'
export type { Terrain } from './terrain'
export type { CanonicalTileID, OverscaledTileID } from './tile_id'

export interface Tile extends Partial<MapLibreTile> {
  tileID?: OverscaledTileID
  rtt?: unknown[] | null
  rttFingerprint?: Record<string, string | number | boolean | null | undefined> | null
  dem?: DEMData
}

export interface MapProjectionTransform {
  getProjectionDataForCustomLayer?: (applyGlobeMatrix?: boolean) => {
    mainMatrix?: ArrayLike<number>
  }
  projectionData?: { mainMatrix?: ArrayLike<number> }
  modelViewProjectionMatrix?: ArrayLike<number>
}

export interface TileManager {
  getRenderableIds(): string[]
  getTileByID(id: string): MapLibreTile | undefined
}

export interface TileManagerContainer {
  tileManagers?: Record<string, TileManager | undefined>
}

export type Map = MapLibreMap & {
  transform: MapProjectionTransform
  style?: TileManagerContainer
  styleManager?: TileManagerContainer
  terrain?: Terrain | null
}

export interface SourceFeatureRecord {
  id?: unknown
}

export interface TileSourceFeatureQuery {
  querySourceFeatures(result: GeoJSONFeature[], params?: QuerySourceFeatureOptions): void
}

export interface RenderableTerrainTile extends Tile {
  tileID: OverscaledTileID
}

export interface TileCoordLike {
  z: number
  x: number
  y: number
}
