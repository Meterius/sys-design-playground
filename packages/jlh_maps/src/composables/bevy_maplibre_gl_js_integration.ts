import {
  create_map_integration,
  remove_features,
  remove_map_integration,
  remove_terrain_tile_data,
  sync_terrain_active_tile_ids,
  sync_view,
  update_features,
  update_terrain_tile_data,
} from 'jlh_maps_app'
import type { GeoJsonProperties, Geometry } from 'geojson'
import type { Map as MapInternal, Tile } from 'maplibre-gl/src/index.ts'
import type { DEMData } from 'maplibre-gl/src/data/dem_data.ts'
import { onWatcherCleanup, toValue, type WatchSource } from 'vue'
import { watchDefinedOnce } from '@/composables/helper.ts'
import { useMap } from '@indoorequal/vue-maplibre-gl'

type TileKey = string

interface TileCoord {
  z: number
  x: number
  y: number
}

interface SyncedFeature {
  key: string
  layer_id: string
  tile_key: TileCoord
  id?: string
  geometry: Geometry
  properties: GeoJsonProperties
}

interface MaplibreGlJsIntegrationOptions {
  featureLayers?: string[]
}

export function useMaplibreGlJsIntegration(
  instanceId: WatchSource<string | undefined>,
  key?: string | symbol,
  options: MaplibreGlJsIntegrationOptions = {},
) {
  const mapInstance = useMap(key)

  watchDefinedOnce(
    () => {
      const instanceIdValue = toValue(instanceId)
      if (instanceIdValue === undefined || mapInstance.map === undefined) return undefined

      return { instanceId: instanceIdValue, map: mapInstance.map }
    },
    ({ instanceId, map }) => {
      const mapIntegrationId = create_map_integration(instanceId)
      const integration = new MaplibreGlJsIntegration(
        map as unknown as MapInternal,
        instanceId,
        mapIntegrationId,
        options.featureLayers ?? [],
      )
      integration.start()

      onWatcherCleanup(() => {
        integration.stop()
        remove_map_integration(instanceId, mapIntegrationId)
      })
    },
  )
}

class MaplibreGlJsIntegration {
  private readonly terrainDataHashes = new Map<TileKey, string>()
  private readonly transmittedFeatureKeys = new Set<string>()
  private syncViewFrame: number | undefined
  private syncDataFrame: number | undefined

  private unsubscribeCallbacks: (() => void)[] = []

  private stopped: boolean = false

  constructor(
    private readonly map: MapInternal,
    private readonly instanceId: string,
    private readonly mapIntegrationId: number,
    private readonly featureLayers: string[],
  ) {}

  start() {
    this.unsubscribeCallbacks.push(...[
      this.on('move', () => this.scheduleSyncView()),
      this.on('zoom', () => this.scheduleSyncView()),
      this.on('rotate', () => this.scheduleSyncView()),
      this.on('pitch', () => this.scheduleSyncView()),
      this.on('resize', () => this.scheduleSyncView()),
      this.on('moveend', () => this.scheduleSyncData()),
      this.on('zoomend', () => this.scheduleSyncData()),
      this.on('rotateend', () => this.scheduleSyncData()),
      this.on('pitchend', () => this.scheduleSyncData()),
      this.on('idle', () => this.scheduleSyncData()),
      this.on('sourcedata', () => this.scheduleSyncData()),
      this.on('styledata', () => this.scheduleSyncData()),
      this.on('data', () => this.scheduleSyncData()),
    ]);

    this.syncView()
    this.syncData()
  }

  stop() {
    if (this.stopped) return;
    this.stopped = true;

    if (this.syncViewFrame !== undefined) {
      cancelAnimationFrame(this.syncViewFrame)
      this.syncViewFrame = undefined
    }
    if (this.syncDataFrame !== undefined) {
      cancelAnimationFrame(this.syncDataFrame)
      this.syncDataFrame = undefined
    }

    this.removeTerrainData([...this.terrainDataHashes.keys()])
    this.removeTransmittedFeatures([...this.transmittedFeatureKeys])
    this.unsubscribeCallbacks.splice(0).forEach((unsubscribe) => unsubscribe())
  }

  private on(type: string, callback: () => void) {
    this.map.on(type, callback)
    return () => this.map.off(type, callback)
  }

  private scheduleSyncView() {
    if (this.syncViewFrame !== undefined) return

    this.syncViewFrame = requestAnimationFrame(() => {
      this.syncViewFrame = undefined
      this.syncView()
    })
  }

  private scheduleSyncData() {
    if (this.syncDataFrame !== undefined) return

    this.syncDataFrame = requestAnimationFrame(() => {
      this.syncDataFrame = undefined
      this.syncData()
    })
  }

  private syncView() {
    const center = this.map.getCenter()
    const canvas = this.map.getCanvas()
    const mainMatrix = this.getMainMatrix()
    if (!mainMatrix) return

    sync_view(
      this.instanceId,
      this.mapIntegrationId,
      canvas.width,
      canvas.height,
      this.map.transform.zoom,
      this.map.getPitch(),
      this.map.getBearing(),
      center.lng,
      center.lat,
      JSON.stringify(mainMatrix),
    )
  }

  private syncData() {
    const visibleTiles = this.getVisibleTiles()
    this.syncFeatures()
    this.syncTerrain()
  }

  private getMainMatrix(): number[] | undefined {
    const transform = this.map.transform as unknown as {
      getProjectionDataForCustomLayer?: () => { mainMatrix?: ArrayLike<number> }
      projectionData?: { mainMatrix?: ArrayLike<number> }
      modelViewProjectionMatrix?: ArrayLike<number>
    }

    const matrix =
      transform.getProjectionDataForCustomLayer?.().mainMatrix ??
      transform.projectionData?.mainMatrix ??
      transform.modelViewProjectionMatrix

    return matrix ? Array.from(matrix) : undefined
  }

  private getVisibleTiles(): Tile[] {
    const tiles = new Map<TileKey, Tile>()

    this.map.terrain?.tileManager?.getRenderableTiles?.().forEach((tile: Tile) => {
      const key = this.getTileKey(tile)
      if (!key) return
      tiles.set(key, tile)
    })

    return [...tiles.values()]
  }

  private syncFeatures() {
    if (this.featureLayers.length === 0) {
      this.removeTransmittedFeatures([...this.transmittedFeatureKeys])
      return
    }

    const features = this.map.queryRenderedFeatures({
      layers: this.featureLayers,
    })

    const syncedFeatures: SyncedFeature[] = features.flatMap((feature, index) => {
      const geojson = feature.toJSON()
      const layerId = feature.layer?.id ?? geojson.layer?.id
      const geometry = geojson.geometry
      if (!layerId || !geometry) return []

      const id = geojson.id == null ? undefined : String(geojson.id)
      const key = id ?? `${layerId}/${index}/${JSON.stringify(geometry)}`

      return [
        {
          key,
          tile_key: { x: feature._x, y: feature._y, z: feature._z },
          layer_id: layerId,
          ...(id == null ? {} : { id }),
          geometry,
          properties: geojson.properties ?? null,
        },
      ]
    })

    const visibleFeatureKeys = new Set(syncedFeatures.map((feature) => feature.key))
    const removedFeatureKeys = [...this.transmittedFeatureKeys].filter(
      (featureKey) => !visibleFeatureKeys.has(featureKey),
    )
    this.removeTransmittedFeatures(removedFeatureKeys)

    const newFeatures = syncedFeatures.filter(
      (feature) => !this.transmittedFeatureKeys.has(feature.key),
    )
    if (newFeatures.length === 0) return

    update_features(this.instanceId, this.mapIntegrationId, JSON.stringify(newFeatures))
    for (const feature of newFeatures) {
      this.transmittedFeatureKeys.add(feature.key)
    }
  }

  private syncTerrain() {
    const terrain = this.map.terrain
    if (!terrain) {
      this.removeTerrainData([...this.terrainDataHashes.keys()])
      return
    }

    const activeTerrainTiles = terrain.tileManager.getRenderableTiles();

    const activeTerrainTileIds = new Set(
      activeTerrainTiles.flatMap((tile) => {
        const key = this.getTileKey(tile)
        return key ? [key] : []
      }),
    )

    this.removeTerrainData(
      [...this.terrainDataHashes.keys()].filter((key) => !activeTerrainTileIds.has(key)),
    )

    sync_terrain_active_tile_ids(this.instanceId, this.mapIntegrationId, [...activeTerrainTileIds]);

    for (const tile of activeTerrainTiles) {
      const key = this.getTileKey(tile)
      if (!key) continue

      const terrainData = terrain.getTerrainData(tile.tileID)
      const dem = terrainData.tile?.dem as DEMData | undefined
      if (!dem) continue
      const hash = this.getTerrainDataHash(tile, terrainData.tile, dem)
      if (this.terrainDataHashes.get(key) === hash) continue

      update_terrain_tile_data(
        this.instanceId,
        this.mapIntegrationId,
        key,
        hash,
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
      this.terrainDataHashes.set(key, hash)
    }
  }

  private removeTransmittedFeatures(featureKeys: string[]) {
    if (featureKeys.length === 0) return

    remove_features(this.instanceId, this.mapIntegrationId, JSON.stringify(featureKeys))
    for (const featureKey of featureKeys) {
      this.transmittedFeatureKeys.delete(featureKey)
    }
  }

  private removeTerrainData(tileKeys: string[]) {
    for (const tileKey of tileKeys) {
      remove_terrain_tile_data(this.instanceId, this.mapIntegrationId, tileKey)
      this.terrainDataHashes.delete(tileKey)
    }
  }

  private getTerrainDataHash(tile: Tile, sourceTile: Tile | null | undefined, dem: DEMData) {
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

  private getTileKey(tile: Tile): TileKey | undefined {
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
