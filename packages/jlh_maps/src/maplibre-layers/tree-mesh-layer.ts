import {
  LngLat,
  LngLatBounds,
  MercatorCoordinate,
  OverscaledTileID,
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  type GeoJSONFeature,
  type Map as MapLibreMap,
  type StyleLayer,
  type Subscription,
} from 'maplibre-gl'
import type { CanonicalTileID } from '@/types/maplibre-gl-internals/tile_id'
import * as THREE from 'three'
import { FBXLoader } from 'three/examples/jsm/loaders/FBXLoader.js'
import { area, bbox, booleanPointInPolygon, point } from '@turf/turf'
import type { Feature, Geometry, MultiPolygon, Polygon, Position } from 'geojson'
import treeCollectionUrl from '../assets/tree-models/source/Lowpoly_Trees_Collection.fbx?url'
import treePaletteUrl from '../assets/tree-models/textures/palette.png?url'

type TileKey = string

interface TileCoord {
  z: number
  x: number
  y: number
}

interface TileEntry {
  tileId: OverscaledTileID
  coord: TileCoord
  group: THREE.Group
  containedFeatures: Set<string>
  inactive: boolean
}

interface TreeMeshLayerOptions {
  maxTrees?: number
  treesPerSquareKm?: number
  minZoom?: number
  scaleMeters?: [number, number]
  inactiveTileCapacity?: number
}

interface TreeInstance {
  position: { x: number; y: number; z: number }
  modelIndex: number
  rotation: number
  horizontalScale: number
  verticalScale: number
}

const TILE_EXTENT = 8192
const DEFAULT_MAX_TREES_PER_TILE = 800
const DEFAULT_TREES_PER_SQUARE_KM = 1200
const DEFAULT_MIN_ZOOM = 14
const DEFAULT_SCALE_METERS: [number, number] = [3.5, 3.5]
const MAX_SAMPLE_ATTEMPTS_PER_TREE = 40

function tileIdToLngLatBounds(tileId: CanonicalTileID) {
  const scale = 2 ** tileId.z
  const west = (tileId.x / scale) * 360 - 180
  const east = ((tileId.x + 1) / scale) * 360 - 180
  const north = mercatorYToLat(tileId.y / scale)
  const south = mercatorYToLat((tileId.y + 1) / scale)

  return new LngLatBounds([west, south], [east, north])
}

function mercatorYToLat(y: number) {
  return (Math.atan(Math.sinh(Math.PI * (1 - 2 * y))) * 180) / Math.PI
}

export class TreeMeshLayer implements CustomLayerInterface {
  id = 'tree-meshes'
  type = 'custom' as const
  renderingMode = '3d' as const

  private map!: MapLibreMap
  private renderer!: THREE.WebGLRenderer
  private camera = new THREE.Camera()
  private scene = new THREE.Scene()
  private models: THREE.Object3D[] = []
  private tileCache = new Map<TileKey, TileEntry>()
  private subscriptions: Subscription[] = []
  private disposed = false
  private modelsReady = false

  private readonly options: Required<TreeMeshLayerOptions>

  constructor(
    private readonly targetLayer: StyleLayer,
    options: TreeMeshLayerOptions = {},
  ) {
    this.options = {
      maxTrees: options.maxTrees ?? DEFAULT_MAX_TREES_PER_TILE,
      treesPerSquareKm: options.treesPerSquareKm ?? DEFAULT_TREES_PER_SQUARE_KM,
      minZoom: options.minZoom ?? DEFAULT_MIN_ZOOM,
      scaleMeters: options.scaleMeters ?? DEFAULT_SCALE_METERS,
      inactiveTileCapacity: options.inactiveTileCapacity ?? 16,
    }
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map

    this.scene.add(new THREE.AmbientLight(0xffffff, 1.5))

    const sun = new THREE.DirectionalLight(0xffffff, 1.2)
    sun.position.set(0, 1, -1)
    this.scene.add(sun)

    this.renderer = new THREE.WebGLRenderer({
      canvas: map.getCanvas(),
      context: gl,
      antialias: true,
    })

    this.renderer.autoClear = false

    this.loadTreeModels().catch(console.error)
  }

  render(
    _gl: WebGLRenderingContext | WebGL2RenderingContext,
    options: CustomRenderMethodInput,
  ): void {
    if (this.disposed) return

    this.buildTiles()

    for (const entry of this.tileCache.values()) {
      if (entry.inactive) continue

      const center = MercatorCoordinate.fromLngLat(
        tileIdToLngLatBounds(entry.tileId.canonical).getCenter(),
      )

      const mainMatrix = new THREE.Matrix4().fromArray(options.defaultProjectionData.mainMatrix)

      const tileMatrix = new THREE.Matrix4()
        .makeTranslation(center.x, center.y, center.z)
        .scale(
          new THREE.Vector3(
            center.meterInMercatorCoordinateUnits(),
            center.meterInMercatorCoordinateUnits(),
            center.meterInMercatorCoordinateUnits(),
          ),
        )

      this.camera.projectionMatrix = mainMatrix.multiply(tileMatrix)

      this.scene.add(entry.group)

      this.renderer.resetState()
      this.renderer.render(this.scene, this.camera)
      this.scene.remove(entry.group)
    }

    this.map.triggerRepaint()
  }

  onRemove(): void {
    this.disposed = true
    this.subscriptions.forEach((subscription) => subscription.unsubscribe())
    this.subscriptions = []

    for (const entry of this.tileCache.values()) {
      this.deleteTileEntry(entry)
    }
    this.tileCache.clear()

    this.models.forEach((model) => this.disposeObject(model))
    this.models = []
    this.renderer?.dispose()
  }

  private async loadTreeModels() {
    const textureLoader = new THREE.TextureLoader()
    const palette = await textureLoader.loadAsync(treePaletteUrl)
    palette.colorSpace = THREE.SRGBColorSpace
    palette.magFilter = THREE.NearestFilter
    palette.minFilter = THREE.NearestFilter

    const source = await new FBXLoader().loadAsync(treeCollectionUrl)
    if (this.disposed) return

    source.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return

      // console.log(object)

      const mesh = object.clone()
      mesh.material = new THREE.MeshLambertMaterial({
        map: palette,
        side: THREE.DoubleSide,
      })
      mesh.castShadow = false
      mesh.receiveShadow = false
      this.normalizeModel(mesh)
      this.models.push(mesh)
    })

    this.modelsReady = true
    this.map.triggerRepaint()
  }

  private normalizeModel(model: THREE.Object3D) {
    model.rotateY(Math.PI / 2)

    const box = new THREE.Box3().setFromObject(model)
    const size = box.getSize(new THREE.Vector3())
    const center = box.getCenter(new THREE.Vector3())
    const maxAxis = Math.max(size.x, size.y, size.z, 1)

    const scaleAdjust = 10 / maxAxis

    model.position.sub(center)
    model.position.z += (size.z / 2) * scaleAdjust
    model.scale.multiplyScalar(scaleAdjust)
  }

  private buildTiles() {
    if (!this.modelsReady) return

    if (this.map.getZoom() < this.options.minZoom) {
      for (const entry of this.tileCache.values()) {
        entry.inactive = true
      }
      this.evictInactiveTiles()
      return
    }

    const grouped = this.groupForestFeaturesByTile()

    for (const entry of this.tileCache.values()) {
      entry.inactive = true
    }

    for (const [key, { features, tileId, coord, containedFeatures }] of grouped) {
      const existing = this.tileCache.get(key)

      if (existing && this.setsEqual(existing.containedFeatures, containedFeatures)) {
        existing.inactive = false
        continue
      }

      if (existing) {
        this.deleteTileEntry(existing)
      }

      const group = this.buildTileGroup(key, features, coord, tileId)
      this.tileCache.set(key, {
        tileId,
        coord,
        group,
        containedFeatures,
        inactive: false,
      })
    }

    this.evictInactiveTiles()
  }

  private groupForestFeaturesByTile() {
    const features = this.map.querySourceFeatures(this.targetLayer.source, {
      sourceLayer: this.targetLayer.sourceLayer,
    })

    const grouped = new Map<
      TileKey,
      {
        features: Array<Feature<Polygon | MultiPolygon>>
        tileId: OverscaledTileID
        coord: TileCoord
        containedFeatures: Set<string>
      }
    >()

    for (const feature of features) {
      const geometry = feature.geometry
      if (!geometry || (geometry.type !== 'Polygon' && geometry.type !== 'MultiPolygon')) continue

      const coord = this.getTileCoord(feature)
      const key = this.getTileKey(coord)

      // if (!(coord.x === 8799 && coord.y === 5373 && coord.z === 14 && feature.id?.toString() === '55')) {
      //   continue;
      // }

      if (!grouped.has(key)) {
        grouped.set(key, {
          features: [],
          tileId: new OverscaledTileID(coord.z, 0, coord.z, coord.x, coord.y),
          coord,
          containedFeatures: new Set(),
        })
      }

      const entry = grouped.get(key)!
      entry.features.push({
        type: 'Feature',
        properties: {},
        geometry,
      })
      entry.containedFeatures.add(this.getFeatureKey(feature))
    }

    return grouped
  }

  private buildTileGroup(
    tileKey: TileKey,
    features: Array<Feature<Polygon | MultiPolygon>>,
    coord: TileCoord,
    tileId: OverscaledTileID,
  ) {
    const group = new THREE.Group()
    group.name = `${this.id}:${tileKey}`

    for (const instance of this.buildTreeInstances(tileKey, features, coord)) {
      const tree = this.createTreeObject(instance, tileId)
      if (tree) group.add(tree)
    }

    // console.log(tileId, group)

    return group
  }

  private buildTreeInstances(
    tileKey: TileKey,
    features: Array<Feature<Polygon | MultiPolygon>>,
    coord: TileCoord,
  ) {
    const featureAreas = features.map((feature) => Math.max(0, area(feature)))
    const totalArea = featureAreas.reduce((sum, value) => sum + value, 0)
    const targetCount = Math.min(
      this.options.maxTrees,
      Math.round((totalArea / 1_000_000) * this.options.treesPerSquareKm),
    )

    if (targetCount <= 0 || totalArea <= 0) return []

    const instances: TreeInstance[] = []
    const random = this.makeRandom(this.hashString(tileKey))

    for (let i = 0; i < features.length; i++) {
      const feature = features[i]!
      const count =
        i === features.length - 1
          ? targetCount - instances.length
          : Math.round(targetCount * (featureAreas[i]! / totalArea))

      instances.push(...this.sampleFeature(feature, coord, Math.max(0, count), random))
    }

    return instances.slice(0, this.options.maxTrees)
  }

  private sampleFeature(
    feature: Feature<Polygon | MultiPolygon>,
    coord: TileCoord,
    count: number,
    random: () => number,
  ) {
    const bounds = bbox(feature)
    const instances: TreeInstance[] = []
    let attempts = 0

    while (instances.length < count && attempts < count * MAX_SAMPLE_ATTEMPTS_PER_TREE) {
      attempts++
      const lng = bounds[0] + random() * (bounds[2] - bounds[0])
      const lat = bounds[1] + random() * (bounds[3] - bounds[1])

      const gcs: [number, number] = [lng, lat]

      if (!booleanPointInPolygon(point(gcs), feature)) continue

      // const position = this.lngLatToTilePoint(lng, lat, coord)
      const elevation =
        this.map.terrain?.getElevationForLngLatZoom(new LngLat(lng, lat), coord.z) ?? 0
      const position = MercatorCoordinate.fromLngLat(gcs, elevation)
      // const scaleMeters =
      //   this.options.scaleMeters[0] +
      //   random() * (this.options.scaleMeters[1] - this.options.scaleMeters[0])

      instances.push({
        position,
        modelIndex: 0,
        // modelIndex: Math.floor(random() * this.models.length),
        rotation: random() * Math.PI * 2,
        horizontalScale: 1.0,
        verticalScale: 1.0,
      })
    }

    return instances
  }

  private createTreeObject(instance: TreeInstance, tileId: OverscaledTileID) {
    const model = this.models[instance.modelIndex % this.models.length]
    if (!model) return undefined

    const tree = model.clone(true)
    tree.rotateOnAxis(new THREE.Vector3(0, 0, 1), instance.rotation)
    //tree.scale.multiplyScalar(15)
    // tree.scale.set(instance.horizontalScale, instance.verticalScale, instance.horizontalScale)

    const center = MercatorCoordinate.fromLngLat(tileIdToLngLatBounds(tileId.canonical).getCenter())
    const scale = 1 / center.meterInMercatorCoordinateUnits()

    const group = new THREE.Group()
    group.position.set(
      scale * (instance.position.x - center.x),
      scale * (instance.position.y - center.y),
      scale * (instance.position.z - center.z),
    )
    // group.position.set(scale * (instance.position.x - center.x), scale * (instance.position.y - center.y), scale * (instance.position.z - center.z))
    //group.scale.setScalar(50)
    // group.rotation.z = instance.rotation
    group.add(tree)

    return group
  }

  private deleteTileEntry(entry: TileEntry) {
    this.scene.remove(entry.group)
    entry.group.clear()
  }

  private evictInactiveTiles() {
    const inactiveTiles = [...this.tileCache].filter(([, entry]) => entry.inactive)
    const deleteCount = inactiveTiles.length - this.options.inactiveTileCapacity

    if (deleteCount <= 0) return

    for (const [key, entry] of inactiveTiles.slice(0, deleteCount)) {
      this.deleteTileEntry(entry)
      this.tileCache.delete(key)
    }
  }

  private disposeObject(object: THREE.Object3D) {
    object.traverse((child) => {
      if (!(child instanceof THREE.Mesh)) return

      child.geometry?.dispose()
      const materials = Array.isArray(child.material) ? child.material : [child.material]
      materials.forEach((material) => material?.dispose())
    })
  }

  private getTileCoord(feature: GeoJSONFeature): TileCoord {
    return {
      z: feature._z,
      x: feature._x,
      y: feature._y,
    }
  }

  private getTileKey(coord: TileCoord): TileKey {
    return `${coord.z}/${coord.x}/${coord.y}`
  }

  private getFeatureKey(feature: GeoJSONFeature) {
    return `${feature.id ?? feature._vectorTileFeature?.id ?? ''}:${this.getGeometryKey(feature.geometry)}`
  }

  private getGeometryKey(geometry: Geometry) {
    return JSON.stringify(this.getFirstPositions(geometry).slice(0, 8))
  }

  private getFirstPositions(geometry: Geometry): Position[] {
    if (geometry.type === 'Polygon') return geometry.coordinates[0] ?? []
    if (geometry.type === 'MultiPolygon') return geometry.coordinates[0]?.[0] ?? []
    return []
  }

  private lngLatToTilePoint(lng: number, lat: number, coord: TileCoord) {
    const scale = 2 ** coord.z
    const worldX = ((lng + 180) / 360) * scale
    const worldY = this.lngLatToMercatorY(lat) * scale

    return {
      x: (worldX - coord.x) * TILE_EXTENT,
      y: (worldY - coord.y) * TILE_EXTENT,
    }
  }

  private metersToTileUnits(lng: number, lat: number, coord: TileCoord) {
    const mercator = MercatorCoordinate.fromLngLat([lng, lat])
    const mercatorUnitsPerTileUnit = 1 / (TILE_EXTENT * 2 ** coord.z)

    return mercator.meterInMercatorCoordinateUnits() / mercatorUnitsPerTileUnit
  }

  private lngLatToMercatorY(lat: number) {
    const radians = (Math.max(-85.051129, Math.min(85.051129, lat)) * Math.PI) / 180
    return (1 - Math.log(Math.tan(radians) + 1 / Math.cos(radians)) / Math.PI) / 2
  }

  private setsEqual(a: Set<string>, b: Set<string>) {
    if (a.size !== b.size) return false
    for (const value of a) {
      if (!b.has(value)) return false
    }
    return true
  }

  private hashString(value: string) {
    let hash = 2166136261
    for (let i = 0; i < value.length; i++) {
      hash ^= value.charCodeAt(i)
      hash = Math.imul(hash, 16777619)
    }
    return hash >>> 0
  }

  private makeRandom(seed: number) {
    let state = seed || 1
    return () => {
      state = Math.imul(1664525, state) + 1013904223
      return (state >>> 0) / 4294967296
    }
  }
}
