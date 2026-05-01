import {
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  type Map as MapLibreMap,
  type MapGeoJSONFeature,
  Point,
} from 'maplibre-gl'
import { OverscaledTileID } from 'maplibre-gl/src/tile/tile_id'
import earcut from 'earcut'
import dynWaterFragShader from '../shaders/dyn_water.frag.glsl?raw'
import dynWaterVertexShader from '../shaders/dyn_water.vertex.glsl?raw'
import { loadGeometry } from 'maplibre-gl/src/data/load_geometry.ts'
import { classifyRings } from '@maplibre/maplibre-gl-style-spec'

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

  private targetLayer = 'Water'

  private tileCache = new Map<TileKey, { mesh: TileMesh; tileId: OverscaledTileID }>()

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

    map.on('moveend', this.rebuild)
    map.on('zoomend', this.rebuild)

    this.rebuild()
  }

  render(gl: WebGLRenderingContext, _options: CustomRenderMethodInput): boolean {
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

  private rebuild = () => {
    const gl = this.gl

    // cleanup old buffers
    for (const { mesh } of this.tileCache.values()) {
      gl.deleteBuffer(mesh.buffer)
    }
    this.tileCache.clear()

    const features = this.map.queryRenderedFeatures({
      layers: [this.targetLayer],
    })

    const grouped = new Map<TileKey, { vertices: number[]; tileId: OverscaledTileID }>()

    for (const feature of features) {
      const key = this.getTileKey(feature)
      if (!key) continue

      if (!grouped.has(key))
        grouped.set(key, {
          vertices: [],
          tileId: new OverscaledTileID(feature._z, 0, feature._z, feature._x, feature._y),
        })
      this.addFeature(grouped.get(key)!.vertices, feature)
    }

    for (const [key, { vertices, tileId }] of grouped) {
      if (vertices.length === 0) continue

      const buffer = gl.createBuffer()!
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices), gl.STATIC_DRAW)

      this.tileCache.set(key, {
        mesh: {
          buffer,
          vertexCount: vertices.length / 2,
        },
        tileId,
      })
    }
  }

  private getTileKey(feature: MapGeoJSONFeature): TileKey | null {
    // z/x/y key
    return `${feature._z}/${feature._x}/${feature._y}`
  }

  private addFeature(out: number[], feature: MapGeoJSONFeature) {
    console.log(
      feature,
      feature._vectorTileFeature.loadGeometry(),
      classifyRings(feature._vectorTileFeature.loadGeometry(), 0),
    )

    classifyRings(loadGeometry(feature._vectorTileFeature), 0).forEach((ring) => {
      this.triangulatePolygon(out, ring)
    })
  }

  private triangulatePolygon(out: number[], rings: Point[][]) {
    const vertices: number[] = []
    const holes: number[] = []
    let holeIndex = 0

    console.log(rings)

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
