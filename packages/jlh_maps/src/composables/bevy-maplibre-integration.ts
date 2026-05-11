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
import type { Geometry } from 'geojson'
import type { Map as MapInternal, Tile } from 'maplibre-gl/src/index.ts'
import type { DEMData } from 'maplibre-gl/src/data/dem_data.ts'
import { onWatcherCleanup, ref, toValue, type WatchSource } from 'vue'
import { watchDefinedOnce } from '@/composables/helper.ts'
import { useMap } from '@indoorequal/vue-maplibre-gl'
import type { Terrain } from 'maplibre-gl/src/render/terrain.ts'
import { CanonicalTileID } from 'maplibre-gl/src/tile/tile_id'

type TileKey = string

interface TileCoord {
  z: number
  x: number
  y: number
}

interface FeatureSourceLayer {
  sourceId: string
  sourceLayer: string
}

interface SyncedFeatureKey {
  source_id: string
  source_layer_id: string
  tile_key: TileCoord
  feature_id: string
}

interface SyncedFeature {
  key: string
  source_id: string
  source_layer_id: string
  tile_key: TileCoord
  feature_id: string
  geometry: Geometry
  properties: Record<string, unknown>
}

interface SourceFeatureRecord {
  _z: number
  _x: number
  _y: number
  id?: unknown
  _vectorTileFeature?: { id?: unknown }
}

interface MaplibreGlJsIntegrationOptions {
  featureSourceLayers?: FeatureSourceLayer[]
}

export function useMaplibreGlJsIntegration(
  instanceId: WatchSource<string | undefined>,
  key?: string | symbol,
  options: MaplibreGlJsIntegrationOptions = {},
) {
  const mapInstance = useMap(key)

  const mapIntegration = ref<MaplibreGlJsIntegration | null>(null)

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
        options.featureSourceLayers ?? [],
      )
      integration.start()

      mapIntegration.value = integration

      onWatcherCleanup(() => {
        integration.stop()
        remove_map_integration(instanceId, mapIntegrationId)
      })
    },
  )

  return {
    syncOnRender: () => mapIntegration.value?.syncOnRender(),
  }
}

class MaplibreGlJsIntegration {
  private readonly terrainDataHashes = new Map<TileKey, string>()
  private readonly transmittedFeatureKeys = new Map<string, SyncedFeatureKey>()
  private syncViewFrame: number | undefined
  private syncDataFrame: number | undefined

  private unsubscribeCallbacks: (() => void)[] = []

  private stopped: boolean = false

  constructor(
    private readonly map: MapInternal,
    private readonly instanceId: string,
    private readonly mapIntegrationId: number,
    private readonly featureSourceLayers: FeatureSourceLayer[],
  ) {}

  start() {
    console.log('Starting maplibre integration on map: ', this.map)

    this.unsubscribeCallbacks.push(
      this.on('moveend', () => this.scheduleSyncData()),
      this.on('zoomend', () => this.scheduleSyncData()),
      this.on('rotateend', () => this.scheduleSyncData()),
      this.on('pitchend', () => this.scheduleSyncData()),
      this.on('idle', () => this.scheduleSyncData()),
      this.on('sourcedata', () => this.scheduleSyncData()),
      this.on('styledata', () => this.scheduleSyncData()),
      this.on('data', () => this.scheduleSyncData()),
    )

    this.syncView()
    this.syncData()
  }

  syncOnRender() {
    this.syncView()
    this.syncTerrain()
  }

  stop() {
    if (this.stopped) return
    this.stopped = true

    if (this.syncViewFrame !== undefined) {
      cancelAnimationFrame(this.syncViewFrame)
      this.syncViewFrame = undefined
    }
    if (this.syncDataFrame !== undefined) {
      cancelAnimationFrame(this.syncDataFrame)
      this.syncDataFrame = undefined
    }

    this.removeTerrainData([...this.terrainDataHashes.keys()])
    this.removeTransmittedFeatures([...this.transmittedFeatureKeys.keys()])
    this.unsubscribeCallbacks.splice(0).forEach((unsubscribe) => unsubscribe())
  }

  private on(type: string, callback: () => void) {
    this.map.on(type, callback)
    return () => this.map.off(type, callback)
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
    this.syncFeatures()
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

  private syncFeatures() {
    if (this.featureSourceLayers.length === 0) {
      this.removeTransmittedFeatures([...this.transmittedFeatureKeys.keys()])
      return
    }

    const syncedFeatures: SyncedFeature[] = this.featureSourceLayers.flatMap(
      ({ sourceId, sourceLayer }) =>
        this.map.querySourceFeatures(sourceId, { sourceLayer }).flatMap((feature, index) => {
          const geojson = feature.toJSON()
          const geometry = geojson.geometry
          if (!geometry) return []

          const sourceFeature = feature as unknown as SourceFeatureRecord
          const tileKey = this.getFeatureTileKey(sourceFeature)
          if (!tileKey) return []

          const featureId = this.getFeatureId(sourceFeature, geometry, index)
          const key = this.getFeatureStorageKey(sourceId, sourceLayer, tileKey, featureId)

          return [
            {
              key,
              source_id: sourceId,
              source_layer_id: sourceLayer,
              tile_key: tileKey,
              feature_id: featureId,
              geometry,
              properties: geojson.properties ?? {},
            },
          ]
        }),
    )

    const visibleFeatureKeys = new Set(syncedFeatures.map((feature) => feature.key))
    const removedFeatureKeys = [...this.transmittedFeatureKeys.keys()].filter(
      (featureKey) => !visibleFeatureKeys.has(featureKey),
    )
    this.removeTransmittedFeatures(removedFeatureKeys)

    const newFeatures = syncedFeatures.filter(
      (feature) => !this.transmittedFeatureKeys.has(feature.key),
    )
    if (newFeatures.length === 0) return

    update_features(this.instanceId, this.mapIntegrationId, JSON.stringify(newFeatures))
    for (const feature of newFeatures) {
      this.transmittedFeatureKeys.set(feature.key, {
        source_id: feature.source_id,
        source_layer_id: feature.source_layer_id,
        tile_key: feature.tile_key,
        feature_id: feature.feature_id,
      })
    }
  }

  private get terrain(): Terrain | null {
    return this.map.terrain
  }

  private syncTerrain() {
    const terrain = this.terrain ?? undefined

    if (terrain) {
      const activeTerrainTiles = terrain.tileManager.getRenderableTiles() ?? []

      const activeTerrainTileIds = new Set(
        activeTerrainTiles.map((tile) => this.getTileKey(tile.tileID.canonical)),
      )

      this.removeTerrainData(
        [...this.terrainDataHashes.keys()].filter((key) => !activeTerrainTileIds.has(key)),
      )

      sync_terrain_active_tile_ids(this.instanceId, this.mapIntegrationId, [
        ...activeTerrainTileIds,
      ])

      for (const tile of activeTerrainTiles) {
        const key = this.getTileKey(tile.tileID.canonical)

        const terrainData = terrain.getTerrainData(tile.tileID)
        const dem = terrainData.tile?.dem
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
    } else {
      // terrain is not available, remove all terrain data that may have existed while terrain was active
      if (this.terrainDataHashes.size !== 0) {
        this.removeTerrainData([...this.terrainDataHashes.keys()])
        this.terrainDataHashes.clear()
      }

      const activeTerrainTileIds = new Set(
        this.map
          .coveringTiles({
            tileSize: 512,
          })
          .map((tileId) => this.getTileKey(tileId.canonical)),
      )

      sync_terrain_active_tile_ids(this.instanceId, this.mapIntegrationId, [
        ...activeTerrainTileIds,
      ])
    }
  }

  private removeTransmittedFeatures(featureKeys: string[]) {
    if (featureKeys.length === 0) return

    const removedFeatures = featureKeys.flatMap((featureKey) => {
      const feature = this.transmittedFeatureKeys.get(featureKey)
      return feature ? [feature] : []
    })
    if (removedFeatures.length !== 0) {
      remove_features(this.instanceId, this.mapIntegrationId, JSON.stringify(removedFeatures))
    }

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

  private getTileKey(tileId: CanonicalTileID): TileKey {
    return `${tileId.z}/${tileId.x}/${tileId.y}`
  }

  private getFeatureTileKey(feature: SourceFeatureRecord): TileCoord | undefined {
    if (
      !Number.isFinite(feature._z) ||
      !Number.isFinite(feature._x) ||
      !Number.isFinite(feature._y)
    ) {
      return undefined
    }

    return {
      z: feature._z,
      x: feature._x,
      y: feature._y,
    }
  }

  private getFeatureId(feature: SourceFeatureRecord, geometry: Geometry, index: number) {
    const id = feature.id ?? feature._vectorTileFeature?.id
    return id == null ? `${index}:${JSON.stringify(geometry)}` : String(id)
  }

  private getFeatureStorageKey(
    sourceId: string,
    sourceLayer: string,
    tileKey: TileCoord,
    featureId: string,
  ) {
    return `${sourceId}/${sourceLayer}/${tileKey.z}/${tileKey.x}/${tileKey.y}/${featureId}`
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
