import type { DEMData } from './dem_data'
import type { RenderableTerrainTile, Tile } from './index'
import type { OverscaledTileID } from './tile_id'

export interface Terrain {
  tileManager: {
    getRenderableTiles(): RenderableTerrainTile[] | undefined
  }
  getTerrainData(tileId: OverscaledTileID): {
    tile?: (Tile & { dem?: DEMData }) | null
    u_terrain_matrix: Iterable<number> | ArrayLike<number>
  }
  getElevationForLngLatZoom?(lngLat: unknown, zoom: number): number | null | undefined
}
