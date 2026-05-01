import {
  type CustomLayerInterface,
  type CustomRenderMethodInput, type GeoJSONFeature,
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

type TileKey = string

interface Edge {
  ax: number
  ay: number
  bx: number
  by: number
}

interface TileMesh {
  buffer: WebGLBuffer
  vertexCount: number
}

interface TileEntry {
  mesh: TileMesh
  tileId: OverscaledTileID
  containedFeatures: Set<string>
  edgeDistanceTexture: WebGLTexture
  edgeDistanceData: Uint8Array
  inactive: boolean
}

const TILE_EXTEND = 8192
const TILE_EDGE_DISTANCE_TEXTURE_SIZE = 256
const TILE_EDGE_DISTANCE_MAX_DISTANCE = 500
const TILE_EDGE_DISTANCE_MAX_BYTE = 255

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

  private targetLayer: StyleLayer

  private tileCache = new Map<TileKey, TileEntry>()
  private tileCacheInactiveCapacity = 8
  private tileCacheBuild = 0

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

    this.buildMeshes()
  }

  render(gl: WebGLRenderingContext, _options: CustomRenderMethodInput): boolean {
    this.buildMeshes()

    gl.useProgram(this.program)

    gl.uniform1f(this.uTime, performance.now() / 1000)
    gl.uniform2f(this.uResolution, gl.canvas.width, gl.canvas.height)
    gl.uniform1i(this.uEdgeDistanceTexture, 0)

    gl.enable(gl.BLEND)
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)

    for (const { mesh, tileId, edgeDistanceTexture, inactive } of this.tileCache.values()) {
      if (inactive) continue

      const proj = this.map.transform.getProjectionData({
        overscaledTileID: tileId,
      })

      gl.uniformMatrix4fv(this.uProjectionMatrix, false, proj.mainMatrix)
      gl.uniform4fv(this.uMercatorCoords, proj.tileMercatorCoords)

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

  private buildMeshes = () => {
    const gl = this.gl
    const activeBuild = ++this.tileCacheBuild

    const features = this.map.querySourceFeatures(this.targetLayer.source, {
      sourceLayer: this.targetLayer.sourceLayer,
    })

    const grouped = new Map<
      TileKey,
      {
        features: GeoJSONFeature[]
        tileId: OverscaledTileID
        containedFeatures: Set<string>
      }
    >()

    for (const feature of features) {
      const key = this.getTileKey(feature)
      if (!key) {
        throw new Error('Missing tile key')
      }

      const tileId = new OverscaledTileID(feature._z, 0, feature._z, feature._x, feature._y)

      if (!grouped.has(key)) {
        grouped.set(key, {
          features: [],
          tileId,
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
      entry.inactive = true;
    }

    // create / update
    for (const [key, { features, tileId, containedFeatures }] of grouped) {
      const existing = this.tileCache.get(key)

      if (existing) {
        existing.inactive = false;
      }

      // check if not exists or features have changed
      const needsRebuild = !existing || !isEqual(existing.containedFeatures, containedFeatures)

      if (!needsRebuild) {
        existing.tileId = tileId
        continue
      }

      // delete existing buffer
      if (existing) {
        gl.deleteBuffer(existing.mesh.buffer)
      }

      const vertices: number[] = []
      const edges: Edge[] = []
      features.forEach((feature) => this.addFeature(vertices, edges, feature))

      const buffer = gl.createBuffer()!
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STATIC_DRAW)

      const edgeDistanceTexture = existing?.edgeDistanceTexture ?? this.createEdgeDistanceTexture()
      const edgeDistanceData =
        existing?.edgeDistanceData ??
        new Uint8Array(TILE_EDGE_DISTANCE_TEXTURE_SIZE * TILE_EDGE_DISTANCE_TEXTURE_SIZE)

      this.updateEdgeDistanceData(edgeDistanceData, edges)
      this.updateEdgeDistanceTexture(edgeDistanceTexture, edgeDistanceData)

      this.tileCache.set(key, {
        mesh: {
          buffer,
          vertexCount: vertices.length / 2,
        },
        tileId,
        containedFeatures,
        edgeDistanceTexture,
        edgeDistanceData,
        inactive: false,
      })
    }

    this.evictInactiveTiles(activeBuild)
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

  private getTileKey(feature: GeoJSONFeature): TileKey | null {
    return `${feature._z}/${feature._x}/${feature._y}`
  }

  private evictInactiveTiles() {
    const inactiveTiles = [...this.tileCache].filter(
      ([, entry]) => entry.inactive
    )
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

  private createEdgeDistanceTexture() {
    const gl = this.gl
    const texture = gl.createTexture()!

    gl.bindTexture(gl.TEXTURE_2D, texture)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1)
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.LUMINANCE,
      TILE_EDGE_DISTANCE_TEXTURE_SIZE,
      TILE_EDGE_DISTANCE_TEXTURE_SIZE,
      0,
      gl.LUMINANCE,
      gl.UNSIGNED_BYTE,
      null,
    )

    return texture
  }

  private updateEdgeDistanceTexture(texture: WebGLTexture, data: Uint8Array) {
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
      gl.LUMINANCE,
      gl.UNSIGNED_BYTE,
      data,
    )
  }

  private updateEdgeDistanceData(out: Uint8Array, edges: Edge[]) {
    const maxDistSq = TILE_EDGE_DISTANCE_MAX_DISTANCE * TILE_EDGE_DISTANCE_MAX_DISTANCE

    for (let y = 0; y < TILE_EDGE_DISTANCE_TEXTURE_SIZE; y++) {
      const rowOffset = y * TILE_EDGE_DISTANCE_TEXTURE_SIZE
      const py = ((y + 0.5) * TILE_EXTEND) / TILE_EDGE_DISTANCE_TEXTURE_SIZE

      for (let x = 0; x < TILE_EDGE_DISTANCE_TEXTURE_SIZE; x++) {
        const px = ((x + 0.5) * TILE_EXTEND) / TILE_EDGE_DISTANCE_TEXTURE_SIZE
        let closestDistSq = maxDistSq

        for (const edge of edges) {
          const distSq = this.distanceToEdgeSq(px, py, edge)

          if (distSq < closestDistSq) {
            closestDistSq = distSq
          }
        }

        const distance = Math.min(Math.sqrt(closestDistSq), TILE_EDGE_DISTANCE_MAX_DISTANCE)
        out[rowOffset + x] = Math.round(
          (distance / TILE_EDGE_DISTANCE_MAX_DISTANCE) * TILE_EDGE_DISTANCE_MAX_BYTE,
        )
      }
    }
  }

  private distanceToEdgeSq(px: number, py: number, edge: Edge) {
    const dx = edge.bx - edge.ax
    const dy = edge.by - edge.ay
    const lenSq = dx * dx + dy * dy

    if (lenSq === 0) {
      const distX = px - edge.ax
      const distY = py - edge.ay

      return distX * distX + distY * distY
    }

    const t = Math.max(0, Math.min(1, ((px - edge.ax) * dx + (py - edge.ay) * dy) / lenSq))
    const closestX = edge.ax + t * dx
    const closestY = edge.ay + t * dy
    const distX = px - closestX
    const distY = py - closestY

    return distX * distX + distY * distY
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
