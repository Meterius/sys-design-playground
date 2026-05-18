import {
  create_map_integration,
  remove_feature_tiles,
  remove_map_integration,
  remove_terrain_tile_data,
  sync_terrain_active_tile_ids,
  sync_view,
  update_feature_tiles,
  update_terrain_tile_data,
} from 'jlh_maps_app'
import type { Geometry } from 'geojson'
import {
  OverscaledTileID,
  type GeoJSONFeature,
  type Map as MapLibreMap,
  type MapSourceDataEvent,
  type QuerySourceFeatureOptions,
  type Tile,
} from 'maplibre-gl'
import { onWatcherCleanup, shallowRef, toValue, type WatchSource } from 'vue'
import { watchDefinedOnce } from '@/composables/helper.ts'
import { useMap } from '@indoorequal/vue-maplibre-gl'
import type {
  CanonicalTileID,
  DEMData,
  Map as InternalMap,
  Terrain,
  Tile as InternalTile,
} from '@/types/maplibre-gl-internals'

type TileKey = string
type SourceLayerTileKey = string

interface TileCoord {
  z: number
  x: number
  y: number
}

interface FeatureSourceLayer {
  sourceId: string
  sourceLayer: string
}

interface SyncedFeatureTileState {
  source_id: string
  source_layer_id: string
  tile_key: TileCoord
  feature_ids: Set<number>
}

interface SyncedFeature {
  feature_id: number
  geometry: Geometry
  properties: Record<string, unknown>
}

interface SyncedFeatureTile {
  source_id: string
  source_layer_id: string
  tile_key: TileCoord
  features: SyncedFeature[]
}

interface RemovedFeatureTile {
  source_id: string
  source_layer_id: string
  tile_key: TileCoord
  feature_ids?: number[]
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

  const mapIntegration = shallowRef<MaplibreGlJsIntegration | null>(null)

  watchDefinedOnce(
    () => {
      const instanceIdValue = toValue(instanceId)
      if (instanceIdValue === undefined || mapInstance.map === undefined) return undefined

      return { instanceId: instanceIdValue, map: mapInstance.map }
    },
    ({ instanceId, map }) => {
      const mapIntegrationId = create_map_integration(instanceId)
      const integration = new MaplibreGlJsIntegration(
        map,
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
  private readonly transmittedFeatureTiles = new Map<SourceLayerTileKey, SyncedFeatureTileState>()
  private readonly pendingChangedFeatureTiles = new Map<
    SourceLayerTileKey,
    { sourceId: string; tileId: OverscaledTileID; tile: Tile }
  >()
  private readonly activeTerrainTileKeys = new Set<TileKey>()
  private readonly featureTerrainTileKeys = new Set<TileKey>()
  private syncViewFrame: number | undefined
  private syncDataFrame: number | undefined

  private unsubscribeCallbacks: (() => void)[] = []

  private stopped: boolean = false

  constructor(
    private readonly map: MapLibreMap,
    private readonly instanceId: string,
    private readonly mapIntegrationId: number,
    private readonly featureSourceLayers: FeatureSourceLayer[],
  ) {}

  start() {
    console.log('Starting maplibre integration on map: ', this.map)

    this.unsubscribeCallbacks.push(
      this.map.on('sourcedata', (event) => this.handleSourceData(event)).unsubscribe,
    )
  }

  syncOnRender() {
    this.syncView()
    this.syncTerrain()
    this.syncVisibleFeatureTiles()
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
    this.removeTransmittedFeatureTiles([...this.transmittedFeatureTiles.keys()])
    this.unsubscribeCallbacks.splice(0).forEach((unsubscribe) => unsubscribe())
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
      this.map.getZoom(),
      this.map.getPitch(),
      this.map.getBearing(),
      center.lng,
      center.lat,
      mainMatrix,
    )
  }

  private syncData() {
    this.syncFeatures()
  }

  private getMainMatrix(): Float64Array | undefined {
    const transform = (this.map as unknown as InternalMap).transform

    const matrix =
      transform.getProjectionDataForCustomLayer?.().mainMatrix ??
      transform.projectionData?.mainMatrix ??
      transform.modelViewProjectionMatrix

    if (!matrix) return undefined
    return matrix instanceof Float64Array ? matrix : new Float64Array(matrix)
  }

  private syncFeatures() {
    if (this.featureSourceLayers.length === 0) {
      this.removeTransmittedFeatureTiles([...this.transmittedFeatureTiles.keys()])
      this.pendingChangedFeatureTiles.clear()
      this.featureTerrainTileKeys.clear()
      this.pruneTerrainData()
      return
    }

    const changedTiles = [...this.pendingChangedFeatureTiles.values()]
    this.pendingChangedFeatureTiles.clear()

    for (const { sourceId, tileId, tile } of changedTiles) {
      this.syncChangedFeatureTile(sourceId, tileId, tile)
    }

    this.refreshFeatureTerrainTileKeys()
    this.pruneTerrainData()
  }

  private handleSourceData(event: MapSourceDataEvent) {
    if (event.dataType !== 'source') return

    const sourceId = event.sourceId
    if (!sourceId || !this.featureSourceLayers.some((layer) => layer.sourceId === sourceId)) {
      return
    }

    const eventTile = event.tile as Tile | undefined
    if (eventTile?.tileID) {
      this.pendingChangedFeatureTiles.set(this.getSourceTileKey(sourceId, eventTile.tileID), {
        sourceId,
        tileId: eventTile.tileID,
        tile: eventTile,
      })
      this.scheduleSyncData()
      return
    }
  }

  private syncChangedFeatureTile(sourceId: string, tileId: OverscaledTileID, tile: Tile) {
    const sourceLayers = this.featureSourceLayers.filter((layer) => layer.sourceId === sourceId)
    for (const { sourceLayer } of sourceLayers) {
      this.syncFeatureTile(sourceId, sourceLayer, tileId, tile)
    }
  }

  private syncVisibleFeatureTiles() {
    const visibleTiles = this.getVisibleFeatureTiles()
    const visibleTileKeys = new Set(visibleTiles.map((tile) => tile.sourceLayerTileKey))
    const removedTileKeys = [...this.transmittedFeatureTiles.keys()].filter(
      (tileKey) => !visibleTileKeys.has(tileKey),
    )
    this.removeTransmittedFeatureTiles(removedTileKeys)

    for (const { sourceId, sourceLayer, tileId, tile, sourceLayerTileKey } of visibleTiles) {
      if (this.transmittedFeatureTiles.has(sourceLayerTileKey)) continue
      this.syncFeatureTile(sourceId, sourceLayer, tileId, tile)
    }
  }

  private syncFeatureTile(
    sourceId: string,
    sourceLayer: string,
    tileId: OverscaledTileID,
    tile: Tile,
  ) {
    const tileCoord = this.getTileCoord(tileId.canonical)
    const sourceLayerTileKey = this.getSourceLayerTileKey(sourceId, sourceLayer, tileId.canonical)
    const previousTile = this.transmittedFeatureTiles.get(sourceLayerTileKey)
    const features = this.queryTileSourceFeatures(tile, sourceLayer)
    const syncedFeatures = new Map<number, SyncedFeature | null>()

    for (const feature of features) {
      const featureId = this.getFeatureId(feature, sourceId, sourceLayer, tileCoord)
      if (featureId === undefined) continue

      if (syncedFeatures.has(featureId)) {
        console.warn('Skipping duplicate MapLibre feature id in source-layer tile', {
          sourceId,
          sourceLayer,
          tileKey: tileCoord,
          featureId,
        })
        continue
      }

      if (previousTile?.feature_ids.has(featureId)) {
        syncedFeatures.set(featureId, null)
        continue
      }

      const geometry = feature.geometry
      if (!geometry) continue

      syncedFeatures.set(featureId, {
        feature_id: featureId,
        geometry,
        properties: feature.properties ?? {},
      })
    }

    const featureIds = new Set(syncedFeatures.keys())
    const removedFeatureIds = previousTile
      ? [...previousTile.feature_ids].filter((featureId) => !featureIds.has(featureId))
      : []
    const newFeatures = [...syncedFeatures.values()].filter(
      (feature): feature is SyncedFeature => feature !== null,
    )

    if (removedFeatureIds.length !== 0) {
      this.removeTransmittedTileFeatureIds(previousTile, removedFeatureIds)
    }

    if (newFeatures.length !== 0) {
      this.syncTerrainDataForTileCoords([tileCoord])

      const featureTile: SyncedFeatureTile = {
        source_id: sourceId,
        source_layer_id: sourceLayer,
        tile_key: tileCoord,
        features: newFeatures,
      }
      update_feature_tiles(this.instanceId, this.mapIntegrationId, [featureTile])
    }

    this.transmittedFeatureTiles.set(sourceLayerTileKey, {
      source_id: sourceId,
      source_layer_id: sourceLayer,
      tile_key: tileCoord,
      feature_ids: featureIds,
    })
  }

  private queryTileSourceFeatures(tile: Tile, sourceLayer: string) {
    const features: GeoJSONFeature[] = []
    const params: QuerySourceFeatureOptions = { sourceLayer, validate: false }
    tile.querySourceFeatures(features, params)
    return features
  }

  private get terrain(): Terrain | null {
    return (this.map as unknown as InternalMap).terrain ?? null
  }

  private syncTerrain() {
    const terrain = this.terrain ?? undefined

    if (terrain) {
      const activeTerrainTiles = terrain.tileManager.getRenderableTiles() ?? []

      const activeTerrainTileIds = new Set(
        activeTerrainTiles.map((tile) => this.getTileKey(tile.tileID.canonical)),
      )
      this.activeTerrainTileKeys.clear()
      for (const key of activeTerrainTileIds) {
        this.activeTerrainTileKeys.add(key)
      }

      this.pruneTerrainData()

      sync_terrain_active_tile_ids(this.instanceId, this.mapIntegrationId, [
        ...activeTerrainTileIds,
      ])

      for (const tile of activeTerrainTiles) {
        const key = this.getTileKey(tile.tileID.canonical)
        this.syncTerrainDataForTileId(key, tile.tileID, tile)
      }
    } else {
      this.activeTerrainTileKeys.clear()
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

  private syncTerrainDataForTileCoords(tileCoords: Iterable<TileCoord>) {
    if (!this.terrain) return

    for (const tileCoord of tileCoords) {
      const tileId = new OverscaledTileID(tileCoord.z, 0, tileCoord.z, tileCoord.x, tileCoord.y)
      this.syncTerrainDataForTileId(this.getTileCoordKey(tileCoord), tileId)
    }
  }

  private syncTerrainDataForTileId(
    tileKey: TileKey,
    tileId: OverscaledTileID,
    renderTile?: InternalTile,
  ) {
    const terrain = this.terrain
    if (!terrain) return

    const terrainData = terrain.getTerrainData(tileId)
    const dem = terrainData.tile?.dem
    if (!dem) return

    const hash = this.getTerrainDataHash(tileKey, renderTile ?? terrainData.tile, dem)
    if (this.terrainDataHashes.get(tileKey) === hash) return

    update_terrain_tile_data(
      this.instanceId,
      this.mapIntegrationId,
      tileKey,
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
    this.terrainDataHashes.set(tileKey, hash)
  }

  private removeTransmittedFeatureTiles(tileKeys: SourceLayerTileKey[]) {
    if (tileKeys.length === 0) return

    const removedFeatureTiles: RemovedFeatureTile[] = tileKeys.flatMap((tileKey) => {
      const tile = this.transmittedFeatureTiles.get(tileKey)
      if (!tile) return []

      return [
        {
          source_id: tile.source_id,
          source_layer_id: tile.source_layer_id,
          tile_key: tile.tile_key,
        },
      ]
    })
    if (removedFeatureTiles.length !== 0) {
      remove_feature_tiles(this.instanceId, this.mapIntegrationId, removedFeatureTiles)
    }

    for (const tileKey of tileKeys) {
      this.transmittedFeatureTiles.delete(tileKey)
    }
  }

  private removeTransmittedTileFeatureIds(
    tile: SyncedFeatureTileState | undefined,
    featureIds: number[],
  ) {
    if (!tile || featureIds.length === 0) return

    remove_feature_tiles(this.instanceId, this.mapIntegrationId, [
      {
        source_id: tile.source_id,
        source_layer_id: tile.source_layer_id,
        tile_key: tile.tile_key,
        feature_ids: featureIds,
      },
    ])
  }

  private removeTerrainData(tileKeys: string[]) {
    for (const tileKey of tileKeys) {
      remove_terrain_tile_data(this.instanceId, this.mapIntegrationId, tileKey)
      this.terrainDataHashes.delete(tileKey)
    }
  }

  private pruneTerrainData() {
    this.removeTerrainData(
      [...this.terrainDataHashes.keys()].filter(
        (key) => !this.activeTerrainTileKeys.has(key) && !this.featureTerrainTileKeys.has(key),
      ),
    )
  }

  private getTerrainDataHash(
    tileKey: TileKey,
    sourceTile: InternalTile | null | undefined,
    dem: DEMData,
  ) {
    const sourceTileID = sourceTile?.tileID?.key ?? sourceTile?.tileID?.toString?.() ?? 'none'
    const rttStamp = sourceTile ? (this.getRttContentStamp(sourceTile) ?? 'none') : 'none'

    return [
      tileKey,
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

  private getTileCoordKey(tileCoord: TileCoord): TileKey {
    return `${tileCoord.z}/${tileCoord.x}/${tileCoord.y}`
  }

  private getTileCoord(tileId: CanonicalTileID): TileCoord {
    return {
      z: tileId.z,
      x: tileId.x,
      y: tileId.y,
    }
  }

  private getFeatureId(
    feature: GeoJSONFeature,
    sourceId: string,
    sourceLayer: string,
    tileKey: TileCoord,
  ): number | undefined {
    const id = feature.id
    if (typeof id === 'number' && Number.isSafeInteger(id) && id >= 0) return id

    console.warn('Skipping MapLibre feature without a numeric id', {
      sourceId,
      sourceLayer,
      tileKey,
      id,
    })
    return undefined
  }

  private getVisibleFeatureTiles() {
    return this.featureSourceLayers.flatMap(({ sourceId, sourceLayer }) => {
      const tileManager = this.getTileManager(sourceId)
      if (!tileManager) return []

      return tileManager.getRenderableIds().flatMap((tileIdKey) => {
        const tile = tileManager.getTileByID(tileIdKey)
        if (!tile?.tileID) return []

        return [
          {
            sourceId,
            sourceLayer,
            tileId: tile.tileID,
            tile,
            sourceLayerTileKey: this.getSourceLayerTileKey(
              sourceId,
              sourceLayer,
              tile.tileID.canonical,
            ),
          },
        ]
      })
    })
  }

  private getTileManager(sourceId: string) {
    const map = this.map as unknown as InternalMap
    return (map.styleManager ?? map.style)?.tileManagers?.[sourceId]
  }

  private getSourceTileKey(sourceId: string, tileId: OverscaledTileID) {
    return `${sourceId}/${this.getTileKey(tileId.canonical)}`
  }

  private getSourceLayerTileKey(sourceId: string, sourceLayer: string, tileId: CanonicalTileID) {
    return `${sourceId}/${sourceLayer}/${this.getTileKey(tileId)}`
  }

  private refreshFeatureTerrainTileKeys() {
    this.featureTerrainTileKeys.clear()
    for (const tile of this.transmittedFeatureTiles.values()) {
      if (tile.feature_ids.size !== 0) {
        this.featureTerrainTileKeys.add(this.getTileCoordKey(tile.tile_key))
      }
    }
  }

  private getRttContentStamp(tile: InternalTile) {
    if (!tile.rtt?.length) return undefined

    const fingerprints = tile.rttFingerprint ? Object.entries(tile.rttFingerprint) : []
    if (fingerprints.length === 0) return undefined

    return fingerprints
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([source, fingerprint]) => `${source}:${fingerprint}`)
      .join('|')
  }
}
