import type { CustomLayerInterface, Map as MapLibreMap } from 'maplibre-gl'
import { toValue, type WatchSource } from 'vue'

interface BevyLayerOptions {
  id?: string
  tick?: () => void
}

const VERTEX_SHADER = `#version 300 es
in vec2 a_pos;
out vec2 v_uv;

void main() {
  v_uv = vec2(a_pos.x * 0.5 + 0.5, 0.5 - a_pos.y * 0.5);
  gl_Position = vec4(a_pos, 0.0, 1.0);
}
`

const FRAGMENT_SHADER = `#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_color_texture;
uniform vec2 u_depth_range;
out vec4 out_color;

void main() {
  vec4 color = texture(u_color_texture, v_uv);

  gl_FragDepth = u_depth_range.x;
  out_color = color;
}
`

export class BevyLayer implements CustomLayerInterface {
  id: string
  type = 'custom' as const
  renderingMode: '2d' | '3d' = '3d'

  private map!: MapLibreMap
  private program: WebGLProgram | undefined
  private texture: WebGLTexture | undefined
  private vertexBuffer: WebGLBuffer | undefined
  private vertexArray: WebGLVertexArrayObject | undefined
  private aPos = -1
  private uColorTexture: WebGLUniformLocation | null = null
  private uDepthRange: WebGLUniformLocation | null = null
  private readonly tickCallback: (() => void) | undefined
  private tickFailed = false

  private textureWidth = 0
  private textureHeight = 0

  constructor(
    private readonly textureCanvas: WatchSource<OffscreenCanvas | null>,
    options: BevyLayerOptions = {},
  ) {
    this.id = options.id ?? 'bevy-texture'
    this.tickCallback = options.tick
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map
    this.program = createProgram(gl, VERTEX_SHADER, FRAGMENT_SHADER)
    this.aPos = gl.getAttribLocation(this.program, 'a_pos')
    this.uColorTexture = gl.getUniformLocation(this.program, 'u_color_texture')
    this.uDepthRange = gl.getUniformLocation(this.program, 'u_depth_range')

    this.texture = createTexture(gl)

    this.vertexBuffer = gl.createBuffer()!
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vertexBuffer)
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
      gl.STATIC_DRAW,
    )

    if (isWebGL2(gl)) {
      this.vertexArray = gl.createVertexArray() ?? undefined
      gl.bindVertexArray(this.vertexArray ?? null)
      gl.enableVertexAttribArray(this.aPos)
      gl.vertexAttribPointer(this.aPos, 2, gl.FLOAT, false, 0, 0)
      gl.bindVertexArray(null)
    }

    gl.bindBuffer(gl.ARRAY_BUFFER, null)
  }

  render(gl: WebGL2RenderingContext | WebGLRenderingContext): void {
    // Run bevy schedule for one frame

    try {
      if (!this.tickFailed) {
        this.tickCallback?.()
      }
    } catch (err) {
      console.error('Error in Bevy layer tick callback:', err)
      this.tickFailed = true
    }

    if (this.tickFailed) {
      return
    }

    const textureCanvas = toValue(this.textureCanvas)

    if (!this.program || !this.vertexBuffer || !this.texture || !textureCanvas) {
      this.map.triggerRepaint()
      return
    }

    // Upload bevy render to maplibre texture

    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, this.texture)
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false)
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true)
    gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE)

    // Recreate texture if dimensions have changed
    if (textureCanvas.width !== this.textureWidth || textureCanvas.height !== this.textureHeight) {
      this.textureWidth = textureCanvas.width
      this.textureHeight = textureCanvas.height

      if (isWebGL2(gl)) {
        gl.deleteTexture(this.texture)
        this.texture = createTexture(gl)
        if (!this.texture) {
          this.map.triggerRepaint()
          return
        }
        gl.bindTexture(gl.TEXTURE_2D, this.texture)
        gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, this.textureWidth, this.textureHeight)
      } else {
        gl.texImage2D(
          gl.TEXTURE_2D,
          0,
          gl.RGBA,
          this.textureWidth,
          this.textureHeight,
          0,
          gl.RGBA,
          gl.UNSIGNED_BYTE,
          null,
        )
      }
    }

    // On chrome this happens < 1ms, likely the current setup is handled as gpu-gpu copy,
    // while on firefox this can take ~20-40ms and incurs a cpu copy
    // TODO: investigate firefox performance bottleneck of texture transfer
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, gl.RGBA, gl.UNSIGNED_BYTE, textureCanvas)

    // Draw bevy render texture as fullscreen quad

    gl.enable(gl.BLEND)
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)
    gl.enable(gl.DEPTH_TEST)
    gl.depthFunc(gl.LEQUAL)
    gl.depthMask(true)

    gl.useProgram(this.program)

    if (isWebGL2(gl) && this.vertexArray) {
      gl.bindVertexArray(this.vertexArray)
    } else {
      gl.bindBuffer(gl.ARRAY_BUFFER, this.vertexBuffer)
      gl.enableVertexAttribArray(this.aPos)
      gl.vertexAttribPointer(this.aPos, 2, gl.FLOAT, false, 0, 0)
    }

    gl.uniform1i(this.uColorTexture, 0)
    gl.uniform2f(
      this.uDepthRange,
      this.map.painter.depthRangeFor3D[0],
      this.map.painter.depthRangeFor3D[1],
    )
    gl.drawArrays(gl.TRIANGLES, 0, 6)

    this.map.triggerRepaint()
  }

  onRemove(_map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    if (this.vertexArray && isWebGL2(gl)) {
      gl.deleteVertexArray(this.vertexArray)
    }
    if (this.vertexBuffer) {
      gl.deleteBuffer(this.vertexBuffer)
    }
    if (this.texture) {
      gl.deleteTexture(this.texture)
    }
    if (this.program) {
      gl.deleteProgram(this.program)
    }
  }
}

function isWebGL2(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
): gl is WebGL2RenderingContext {
  return 'createVertexArray' in gl
}

function createTexture(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
): WebGLTexture | undefined {
  const texture = gl.createTexture() ?? undefined
  gl.bindTexture(gl.TEXTURE_2D, texture ?? null)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
  gl.bindTexture(gl.TEXTURE_2D, null)
  return texture
}

function createProgram(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
  vertexShaderSource: string,
  fragmentShaderSource: string,
) {
  const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource)
  const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource)
  const program = gl.createProgram()
  if (!program) throw new Error('Failed to create Bevy layer program')

  gl.attachShader(program, vertexShader)
  gl.attachShader(program, fragmentShader)
  gl.linkProgram(program)
  gl.deleteShader(vertexShader)
  gl.deleteShader(fragmentShader)

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(program) || 'Unknown Bevy layer program error'
    gl.deleteProgram(program)
    throw new Error(info)
  }

  return program
}

function createShader(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
  type: number,
  source: string,
) {
  const shader = gl.createShader(type)
  if (!shader) throw new Error('Failed to create Bevy layer shader')

  gl.shaderSource(shader, source)
  gl.compileShader(shader)

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader) || 'Unknown Bevy layer shader error'
    gl.deleteShader(shader)
    throw new Error(info)
  }

  return shader
}
