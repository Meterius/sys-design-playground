import {
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  type GeoJSONFeature,
  type Map as MapLibreMap,
  Point,
  StyleLayer,
} from 'maplibre-gl'
import { OverscaledTileID } from 'maplibre-gl/src/tile/tile_id'
import earcut from 'earcut'
import dynWaterFragShader from '../shaders/dyn_water.frag.glsl?raw'
import dynWaterVertexShader from '../shaders/dyn_water.vertex.glsl?raw'
import { loadGeometry } from 'maplibre-gl/src/data/load_geometry.ts'
import { classifyRings } from '@maplibre/maplibre-gl-style-spec'
import { isEqual } from 'lodash'
import initJlhMapsFrontendWasm, { update_edge_distance_texture } from 'jlh_maps_frontend'

type TileKey = string

interface Edge {
  ax: number
  ay: number
  bx: number
  by: number
}

interface TileCoord {
  z: number
  x: number
  y: number
}

interface TileMesh {
  buffer: WebGLBuffer
  vertexCount: number
}

interface DynTerrainData {
  u_terrain_dim: number
  u_terrain_matrix: unknown
  u_terrain_unpack: number[]
  u_terrain_exaggeration: number
  texture: WebGLTexture
}

interface TerrainLike {
  getTerrainData(tileID: OverscaledTileID): DynTerrainData
}

type TerrainAwareMap = {
  terrain?: TerrainLike | null
}

interface TileEntry {
  mesh: TileMesh
  tileId: OverscaledTileID
  coord: TileCoord
  containedFeatures: Set<string>
  edges: Edge[]
  edgeDistanceTexture: WebGLTexture
  edgeDistanceData: Float32Array
  inactive: boolean
}

const TILE_EXTEND = 8192
const TILE_EDGE_DISTANCE_TEXTURE_SIZE = 1024
const TILE_EDGE_DISTANCE_MAX_DISTANCE = 0.0075

export class DynWaterLayer implements CustomLayerInterface {
  id = 'dyn-water'
  type = 'custom' as const
  renderingMode = '2d' as const

  private map!: MapLibreMap
  private gl!: WebGLRenderingContext
  private program!: WebGLProgram

  private aPos!: number
  private uProjectionMatrix!: WebGLUniformLocation
  private uMercatorCoords!: WebGLUniformLocation
  private uResolution!: WebGLUniformLocation
  private uTime!: WebGLUniformLocation
  private uEdgeDistanceTexture!: WebGLUniformLocation
  private uHasTerrain!: WebGLUniformLocation
  private uTerrainTexture!: WebGLUniformLocation
  private uTerrainDim!: WebGLUniformLocation
  private uTerrainMatrix!: WebGLUniformLocation
  private uTerrainUnpack!: WebGLUniformLocation
  private uTerrainExaggeration!: WebGLUniformLocation

  private targetLayer: StyleLayer

  private tileCache = new Map<TileKey, TileEntry>()
  private tileCacheInactiveCapacity = 8
  private edgeDistanceTextureFilter!: number
  private edgeDistanceTextureInternalFormat!: number
  private edgeDistanceTextureFormat!: number
  private wasmReady = false

  constructor(targetLayer: StyleLayer) {
    this.targetLayer = targetLayer
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext): void {
    this.map = map
    this.gl = gl

    const compile = (type: number, src: string) => {
      const s = gl.createShader(type)!
      gl.shaderSource(s, src)
      gl.compileShader(s)
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
        throw new Error(gl.getShaderInfoLog(s) || '')
      }
      return s
    }

    const program = gl.createProgram()!
    gl.attachShader(program, compile(gl.VERTEX_SHADER, dynWaterVertexShader))
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, dynWaterFragShader))
    gl.linkProgram(program)

    this.program = program

    this.aPos = gl.getAttribLocation(program, 'a_pos')
    this.uProjectionMatrix = gl.getUniformLocation(program, 'u_projection_matrix')!
    this.uMercatorCoords = gl.getUniformLocation(program, 'u_projection_tile_mercator_coords')!
    this.uResolution = gl.getUniformLocation(program, 'u_resolution')!
    this.uTime = gl.getUniformLocation(program, 'u_time')!
    this.uEdgeDistanceTexture = gl.getUniformLocation(program, 'u_edge_distance_texture')!
    this.uHasTerrain = gl.getUniformLocation(program, 'u_has_terrain')!
    this.uTerrainTexture = gl.getUniformLocation(program, 'u_terrain')!
    this.uTerrainDim = gl.getUniformLocation(program, 'u_terrain_dim')!
    this.uTerrainMatrix = gl.getUniformLocation(program, 'u_terrain_matrix')!
    this.uTerrainUnpack = gl.getUniformLocation(program, 'u_terrain_unpack')!
    this.uTerrainExaggeration = gl.getUniformLocation(program, 'u_terrain_exaggeration')!

    this.configureEdgeDistanceTextureFormat(gl)
    this.edgeDistanceTextureFilter = gl.getExtension('OES_texture_float_linear')
      ? gl.LINEAR
      : gl.NEAREST

    void initJlhMapsFrontendWasm().then(() => {
      this.wasmReady = true
      this.buildMeshes()
      this.map.triggerRepaint()
    })
  }

  render(gl: WebGLRenderingContext, _options: CustomRenderMethodInput): boolean {
    if (!this.wasmReady) {
      this.map.triggerRepaint()
      return true
    }

    this.buildMeshes()

    gl.useProgram(this.program)

    gl.uniform1f(this.uTime, performance.now() / 1000)
    gl.uniform2f(this.uResolution, gl.canvas.width, gl.canvas.height)
    gl.uniform1i(this.uEdgeDistanceTexture, 0)
    gl.uniform1i(this.uTerrainTexture, 1)

    gl.enable(gl.BLEND)
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA)

    const terrain = (this.map as unknown as TerrainAwareMap).terrain

    for (const { mesh, tileId, edgeDistanceTexture, inactive } of this.tileCache.values()) {
      if (inactive) continue

      const proj = this.map.transform.getProjectionData({
        overscaledTileID: tileId,
      })

      gl.uniformMatrix4fv(this.uProjectionMatrix, false, proj.mainMatrix)
      gl.uniform4fv(this.uMercatorCoords, proj.tileMercatorCoords)
      this.bindTerrainData(terrain?.getTerrainData(tileId))

      gl.activeTexture(gl.TEXTURE0)
      gl.bindTexture(gl.TEXTURE_2D, edgeDistanceTexture)

      gl.bindBuffer(gl.ARRAY_BUFFER, mesh.buffer)

      gl.enableVertexAttribArray(this.aPos)
      gl.vertexAttribPointer(this.aPos, 2, gl.FLOAT, false, 0, 0)

      gl.drawArrays(gl.TRIANGLES, 0, mesh.vertexCount)
    }

    this.map.triggerRepaint()

    return true
  }

  private bindTerrainData(terrainData: DynTerrainData | undefined) {
    const gl = this.gl

    if (!terrainData) {
      gl.uniform1i(this.uHasTerrain, 0)
      return
    }

    gl.uniform1i(this.uHasTerrain, 1)
    gl.uniform1f(this.uTerrainDim, terrainData.u_terrain_dim)
    gl.uniformMatrix4fv(this.uTerrainMatrix, false, terrainData.u_terrain_matrix as Float32List)
    gl.uniform4f(
      this.uTerrainUnpack,
      terrainData.u_terrain_unpack[0] ?? 0,
      terrainData.u_terrain_unpack[1] ?? 0,
      terrainData.u_terrain_unpack[2] ?? 0,
      terrainData.u_terrain_unpack[3] ?? 0,
    )
    gl.uniform1f(this.uTerrainExaggeration, terrainData.u_terrain_exaggeration)
    gl.activeTexture(gl.TEXTURE1)
    gl.bindTexture(gl.TEXTURE_2D, terrainData.texture)
  }

  private buildMeshes = () => {
    if (!this.wasmReady) return

    const gl = this.gl

    const features = this.map.querySourceFeatures(this.targetLayer.source, {
      sourceLayer: this.targetLayer.sourceLayer,
    })

    const grouped = new Map<
      TileKey,
      {
        features: GeoJSONFeature[]
        tileId: OverscaledTileID
        coord: TileCoord
        containedFeatures: Set<string>
      }
    >()

    for (const feature of features) {
      const coord = this.getTileCoord(feature)
      const key = this.getTileKey(coord)

      const tileId = new OverscaledTileID(coord.z, 0, coord.z, coord.x, coord.y)

      if (!grouped.has(key)) {
        grouped.set(key, {
          features: [],
          tileId,
          coord,
          containedFeatures: new Set(),
        })
      }

      const entry = grouped.get(key)!
      const featureId = feature.id ?? feature._vectorTileFeature?.id

      if (featureId === undefined) {
        throw new Error('Missing feature id')
      }

      entry.features.push(feature)
      entry.containedFeatures.add(featureId.toString())
    }

    for (const entry of this.tileCache.values()) {
      entry.inactive = true
    }

    const rebuiltTiles = new Set<TileKey>()

    // create / update
    for (const [key, { features, tileId, coord, containedFeatures }] of grouped) {
      const existing = this.tileCache.get(key)

      if (existing) {
        existing.inactive = false
      }

      // check if not exists or features have changed
      const needsRebuild = !existing || !isEqual(existing.containedFeatures, containedFeatures)

      if (!needsRebuild) {
        continue
      }

      // delete existing buffer
      if (existing) {
        gl.deleteBuffer(existing.mesh.buffer)
      }

      // console.log(key, features, tileId)

      const vertices: number[] = []
      const edges: Edge[] = []
      features.forEach((feature) => this.addFeature(vertices, edges, feature))

      const buffer = gl.createBuffer()!
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STATIC_DRAW)

      const edgeDistanceTexture = existing?.edgeDistanceTexture ?? this.createEdgeDistanceTexture()
      const edgeDistanceData =
        existing?.edgeDistanceData ??
        new Float32Array(TILE_EDGE_DISTANCE_TEXTURE_SIZE * TILE_EDGE_DISTANCE_TEXTURE_SIZE)

      this.tileCache.set(key, {
        mesh: {
          buffer,
          vertexCount: vertices.length / 2,
        },
        tileId,
        coord,
        containedFeatures,
        edges,
        edgeDistanceTexture,
        edgeDistanceData,
        inactive: false,
      })

      rebuiltTiles.add(key)
    }

    this.updateEdgeDistanceTextures(rebuiltTiles)
    this.evictInactiveTiles()
  }

  onRemove(): void {
    const gl = this.gl

    for (const entry of this.tileCache.values()) {
      this.deleteTileEntry(entry)
    }

    this.tileCache.clear()

    if (this.program) {
      gl.deleteProgram(this.program)
    }
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

  private evictInactiveTiles() {
    const inactiveTiles = [...this.tileCache].filter(([, entry]) => entry.inactive)
    const deleteCount = inactiveTiles.length - this.tileCacheInactiveCapacity

    if (deleteCount <= 0) return

    for (const [key, entry] of inactiveTiles.slice(0, deleteCount)) {
      this.deleteTileEntry(entry)
      this.tileCache.delete(key)
    }
  }

  private deleteTileEntry(entry: TileEntry) {
    const gl = this.gl

    gl.deleteBuffer(entry.mesh.buffer)
    gl.deleteTexture(entry.edgeDistanceTexture)
  }

  private addFeature(outVertexBuffer: number[], outEdges: Edge[], feature: GeoJSONFeature) {
    classifyRings(loadGeometry(feature._vectorTileFeature), 0).forEach((ring) => {
      this.triangulatePolygon(outVertexBuffer, ring)
      this.addPolygonEdges(outEdges, ring)
    })
  }

  private addPolygonEdges(outEdges: Edge[], rings: Point[][]) {
    for (const ring of rings) {
      for (let i = 0; i < ring.length; i++) {
        const a = ring[i]!
        const b = ring[(i + 1) % ring.length]!

        if (a.x === b.x && a.y === b.y) continue

        outEdges.push({
          ax: a.x,
          ay: a.y,
          bx: b.x,
          by: b.y,
        })
      }
    }
  }

  private updateEdgeDistanceTextures(rebuiltTiles: Set<TileKey>) {
    for (const [key, entry] of this.tileCache) {
      const needsUpdate = rebuiltTiles.has(key)

      if (!needsUpdate) {
        continue
      }

      const edges = this.collectEdgeDistanceEdges(entry)
      this.updateEdgeDistanceData(entry.edgeDistanceData, edges)
      this.updateEdgeDistanceTexture(entry.edgeDistanceTexture, entry.edgeDistanceData)
    }
  }

  private collectEdgeDistanceEdges(target: TileEntry) {
    const edges: Edge[] = []

    this.addTransformedEdges(edges, target, target)
    return this.removeDuplicateEdges(edges)
  }

  private addTransformedEdges(outEdges: Edge[], target: TileEntry, source: TileEntry) {
    const offsetX = source.coord.x - target.coord.x
    const offsetY = source.coord.y - target.coord.y

    for (const edge of source.edges) {
      const ax = offsetX + edge.ax / TILE_EXTEND
      const ay = offsetY + edge.ay / TILE_EXTEND
      const bx = offsetX + edge.bx / TILE_EXTEND
      const by = offsetY + edge.by / TILE_EXTEND

      outEdges.push({ ax, ay, bx, by })
    }
  }

  private removeDuplicateEdges(edges: Edge[]) {
    const edgeCounts = new Map<string, number>()

    for (const edge of edges) {
      const key = this.getCanonicalEdgeKey(edge)
      edgeCounts.set(key, (edgeCounts.get(key) ?? 0) + 1)
    }

    return edges.filter((edge) => edgeCounts.get(this.getCanonicalEdgeKey(edge)) === 1)
  }

  private getCanonicalEdgeKey(edge: Edge) {
    const ax = this.quantizeNormalizedEdgeCoord(edge.ax)
    const ay = this.quantizeNormalizedEdgeCoord(edge.ay)
    const bx = this.quantizeNormalizedEdgeCoord(edge.bx)
    const by = this.quantizeNormalizedEdgeCoord(edge.by)

    if (ax < bx || (ax === bx && ay <= by)) {
      return `${ax},${ay}:${bx},${by}`
    }

    return `${bx},${by}:${ax},${ay}`
  }

  private quantizeNormalizedEdgeCoord(value: number) {
    return Math.round(0.25 * value * TILE_EXTEND)
  }

  private configureEdgeDistanceTextureFormat(gl: WebGLRenderingContext) {
    if (this.isWebGL2(gl)) {
      this.edgeDistanceTextureInternalFormat = gl.R32F
      this.edgeDistanceTextureFormat = gl.RED
      return
    }

    if (!gl.getExtension('OES_texture_float')) {
      throw new Error(
        'DynWaterLayer requires WebGL2 or OES_texture_float for edge distance textures',
      )
    }

    this.edgeDistanceTextureInternalFormat = gl.LUMINANCE
    this.edgeDistanceTextureFormat = gl.LUMINANCE
  }

  private isWebGL2(gl: WebGLRenderingContext): gl is WebGL2RenderingContext {
    return typeof WebGL2RenderingContext !== 'undefined' && gl instanceof WebGL2RenderingContext
  }

  private createEdgeDistanceTexture() {
    const gl = this.gl
    const texture = gl.createTexture()!

    gl.bindTexture(gl.TEXTURE_2D, texture)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, this.edgeDistanceTextureFilter)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, this.edgeDistanceTextureFilter)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1)
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      this.edgeDistanceTextureInternalFormat,
      TILE_EDGE_DISTANCE_TEXTURE_SIZE,
      TILE_EDGE_DISTANCE_TEXTURE_SIZE,
      0,
      this.edgeDistanceTextureFormat,
      gl.FLOAT,
      null,
    )

    return texture
  }

  private updateEdgeDistanceTexture(texture: WebGLTexture, data: Float32Array) {
    const gl = this.gl

    gl.bindTexture(gl.TEXTURE_2D, texture)
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1)
    gl.texSubImage2D(
      gl.TEXTURE_2D,
      0,
      0,
      0,
      TILE_EDGE_DISTANCE_TEXTURE_SIZE,
      TILE_EDGE_DISTANCE_TEXTURE_SIZE,
      this.edgeDistanceTextureFormat,
      gl.FLOAT,
      data,
    )
  }

  private updateEdgeDistanceData(out: Float32Array, edges: Edge[]) {
    update_edge_distance_texture(
      this.packNormalizedEdges(edges),
      out,
      TILE_EDGE_DISTANCE_TEXTURE_SIZE,
      TILE_EDGE_DISTANCE_MAX_DISTANCE,
    )
  }

  private packNormalizedEdges(edges: Edge[]) {
    const packed = new Float32Array(edges.length * 4)

    for (let i = 0; i < edges.length; i++) {
      const edge = edges[i]!
      const offset = i * 4

      packed[offset] = edge.ax
      packed[offset + 1] = edge.ay
      packed[offset + 2] = edge.bx
      packed[offset + 3] = edge.by
    }

    return packed
  }

  private triangulatePolygon(out: number[], rings: Point[][]) {
    const vertices: number[] = []
    const holes: number[] = []
    let holeIndex = 0

    for (let i = 0; i < rings.length; i++) {
      if (i > 0) {
        holeIndex += rings[i - 1]!.length
        holes.push(holeIndex)
      }

      for (const coord of rings[i]!) {
        vertices.push(coord.x, coord.y)
      }
    }

    const indices = earcut(vertices, holes, 2)

    for (let i = 0; i < indices.length; i++) {
      const idx = indices[i]! * 2
      out.push(vertices[idx]!, vertices[idx + 1]!)
    }
  }
}
