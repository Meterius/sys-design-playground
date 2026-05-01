import {
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  type Map as MapLibreMap,
  type MapGeoJSONFeature,
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

interface TileMesh {
  buffer: WebGLBuffer
  vertexCount: number
}

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

  private targetLayer: StyleLayer

  private tileCache = new Map<
    TileKey,
    {
      mesh: TileMesh
      tileId: OverscaledTileID
      containedFeatures: Set<string>
    }
  >()

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

    this.buildMeshes()
  }

  render(gl: WebGLRenderingContext, _options: CustomRenderMethodInput): boolean {
    this.buildMeshes()

    gl.useProgram(this.program)

    gl.uniform1f(this.uTime, performance.now() / 1000)
    gl.uniform2f(this.uResolution, gl.canvas.width, gl.canvas.height)

    gl.enable(gl.BLEND)
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)

    for (const { mesh, tileId } of this.tileCache.values()) {
      const proj = this.map.transform.getProjectionData({
        overscaledTileID: tileId,
      })

      gl.uniformMatrix4fv(this.uProjectionMatrix, false, proj.mainMatrix)
      gl.uniform4fv(this.uMercatorCoords, proj.tileMercatorCoords)

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

    const features = this.map.queryRenderedFeatures({
      layers: [this.targetLayer.id],
    })

    const grouped = new Map<
      TileKey,
      {
        features: MapGeoJSONFeature[]
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

    // create / update
    for (const [key, { features, tileId, containedFeatures }] of grouped) {
      const existing = this.tileCache.get(key)

      // check if not exists or features have changed
      const needsRebuild = !existing || !isEqual(existing.containedFeatures, containedFeatures)

      if (!needsRebuild) continue

      // delete existing buffer
      if (existing) {
        gl.deleteBuffer(existing.mesh.buffer)
      }

      const vertices: number[] = []
      features.forEach((feature) => this.addFeature(vertices, feature))

      const buffer = gl.createBuffer()!
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STATIC_DRAW)

      this.tileCache.set(key, {
        mesh: {
          buffer,
          vertexCount: vertices.length / 2,
        },
        tileId,
        containedFeatures,
      })
    }

    // delete stale
    for (const key of this.tileCache.keys()) {
      if (!grouped.has(key)) {
        const entry = this.tileCache.get(key)!
        gl.deleteBuffer(entry.mesh.buffer)
        this.tileCache.delete(key)
      }
    }
  }

  private getTileKey(feature: MapGeoJSONFeature): TileKey | null {
    return `${feature._z}/${feature._x}/${feature._y}`
  }

  private addFeature(out: number[], feature: MapGeoJSONFeature) {
    classifyRings(loadGeometry(feature._vectorTileFeature), 0).forEach((ring) => {
      this.triangulatePolygon(out, ring)
    })
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
