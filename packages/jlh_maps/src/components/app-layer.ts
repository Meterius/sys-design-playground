import {
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  EXTENT,
  type Map as MapLibreMap,
  type Subscription,
} from 'maplibre-gl'
import { tileIdToLngLatBounds } from 'maplibre-gl/src/tile/tile_id_to_lng_lat_bounds.ts'
import { sync_terrain_data, sync_tiles, sync_view } from 'jlh_maps_app'
import type { Map as MapInternal, Tile } from 'maplibre-gl/src/index.ts'
import type { DEMData } from 'maplibre-gl/src/data/dem_data.ts'

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

export class AppLayer implements CustomLayerInterface {
  id = 'app-layer'
  type = 'custom' as const
  renderingMode = '3d' as const

  private readonly terrainDataStamps = new Map<TileKey, string>()
  private map!: MapInternal
  private readFramebuffer: WebGLFramebuffer | null = null

  private subscriptions: Subscription[] = []

  constructor(private readonly canvasSelector: string) {
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map as unknown as MapInternal
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
    // this.syncTerrainRenderTextures(gl)
    this.syncTerrainData()
  }

  onRemove(_map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    if (this.readFramebuffer) {
      gl.deleteFramebuffer(this.readFramebuffer)
      this.readFramebuffer = null
    }
    this.terrainDataStamps.clear()
    this.subscriptions.splice(0).forEach(subscription => subscription.unsubscribe())
  }

  private getVisibleTiles() {
    const tiles = new Map<TileKey, SyncedTile>()

    const tileManagers = [this.map.style.tileManagers['terrain']!];// Object.values(this.map.style.tileManagers);
    //
    // tileManagers.forEach((tileManager) => {
    //   tileManager.getRenderableIds().map((id) => {
    //     const tile = tileManager.getTileByID(id)!;
    //     const key = tile.tileID.canonical
    //     const bounds = tileIdToLngLatBounds(key)
    //     tiles.set(`${key.z}/${key.x}/${key.y}`, {
    //       key: { z: key.z, x: key.x, y: key.y },
    //       bounds_lnglat: [
    //         [bounds.getWest(), bounds.getSouth()],
    //         [bounds.getEast(), bounds.getNorth()],
    //       ],
    //     })
    //   })
    // })
    //

    this.map.terrain?.tileManager?.getRenderableTiles?.().forEach((tile: Tile) => {
      const key = tile.tileID?.canonical
      if (!key) return
      const bounds = tileIdToLngLatBounds(key)
      tiles.set(`${key.z}/${key.x}/${key.y}`, {
        key: { z: key.z, x: key.x, y: key.y },
        bounds_lnglat: [
          [bounds.getWest(), bounds.getSouth()],
          [bounds.getEast(), bounds.getNorth()],
        ],
      })
    })

    return [...tiles.values()]
  }

  private syncTerrainData() {
    const terrain = this.map.terrain
    const tiles = terrain?.tileManager?.getRenderableTiles?.() ?? []

    for (const tile of tiles) {
      const key = this.getRttTileKey(tile)
      if (!key) continue

      const terrainData = terrain.getTerrainData(tile.tileID)
      const dem = terrainData.tile?.dem as DEMData | undefined
      if (!dem) continue
      const contentStamp = this.getTerrainDataContentStamp(tile, terrainData.tile, dem)
      if (this.terrainDataStamps.get(key) === contentStamp) continue

      sync_terrain_data(
        this.canvasSelector,
        key,
        dem.stride,
        dem.dim,
        dem.min,
        dem.max,
        dem.redFactor,
        dem.greenFactor,
        dem.blueFactor,
        dem.baseShift,
        JSON.stringify(Array.from(terrainData.u_terrain_matrix)),
        new Uint32Array(dem.data),
      )
      this.terrainDataStamps.set(key, contentStamp)
    }

    // const tileKeys = new Set(tiles.map((tile) => this.getRttTileKey(tile)))
    // for (const key of this.terrainDataStamps.keys()) {
    //   if (!tileKeys.has(key)) {
    //     this.terrainDataStamps.delete(key)
    //   }
    // }
  }

  private getTerrainDataContentStamp(tile: Tile, sourceTile: Tile | null | undefined, dem: DEMData) {
    const sourceTileID = sourceTile?.tileID?.key ?? sourceTile?.tileID?.toString?.() ?? 'none'
    const renderTileID = tile.tileID?.key ?? tile.tileID?.toString?.() ?? 'none'
    const rttStamp = this.getRttContentStamp(tile) ?? 'none'

    return [
      renderTileID,
      sourceTileID,
      dem.uid,
      dem.stride,
      dem.dim,
      dem.min,
      dem.max,
      dem.redFactor,
      dem.greenFactor,
      dem.blueFactor,
      dem.baseShift,
      rttStamp,
    ].join('|')
  }

  private getRttTileKey(tile: Tile): TileKey | undefined {
    const canonical = tile.tileID?.canonical
    if (!canonical) return undefined

    return `${canonical.z}/${canonical.x}/${canonical.y}`
  }

  private getRttContentStamp(tile: Tile) {
    if (!tile.rtt?.length) return undefined

    const fingerprints = tile.rttFingerprint ? Object.entries(tile.rttFingerprint) : []
    if (fingerprints.length === 0) return undefined

    return fingerprints
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([source, fingerprint]) => `${source}:${fingerprint}`)
      .join('|')
  }
}
