import type { CustomLayerInterface, Map as MapLibreMap } from 'maplibre-gl'

type TextureProvider = WebGLTexture | (() => WebGLTexture | undefined | null)
export type TextureLayerDepthMode = 'texture' | 'front' | 'back'

interface TextureLayerOptions {
  id?: string
  depthMode?: TextureLayerDepthMode
  depthTexture?: TextureProvider
}

const DEPTH_VERTEX_SHADER = `#version 300 es
in vec2 a_pos;
out vec2 v_uv;

void main() {
  v_uv = vec2(a_pos.x * 0.5 + 0.5, 0.5 - a_pos.y * 0.5);
  gl_Position = vec4(a_pos, 0.0, 1.0);
}
`

const DEPTH_FRAGMENT_SHADER = `#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_color_texture;
uniform sampler2D u_depth_texture;
uniform int u_depth_mode;
uniform bool u_has_depth_texture;
out vec4 out_color;

void main() {
  vec4 color = texture(u_color_texture, v_uv);

  // if (color.a <= 0.000001 && dot(color.rgb, color.rgb) <= 0.000001) {
  //   discard;
  // }

  float depth = u_has_depth_texture ? mix(0.94, 0.944244384765625, texture(u_depth_texture, v_uv).r) : 1.0 - color.a;
  // if (u_depth_mode == 1) {
  //   depth = 0.0;
  // } else if (u_depth_mode == 2) {
  //   depth = 1.0;
  // }

  gl_FragDepth = clamp(depth, 0.0, 1.0);
  out_color = vec4(vec3(depth), 1.0);
}
`

export class TextureLayer implements CustomLayerInterface {
  id: string
  type = 'custom' as const
  renderingMode: '2d' | '3d'

  private map!: MapLibreMap
  private program: WebGLProgram | undefined
  private vertexBuffer: WebGLBuffer | undefined
  private vertexArray: WebGLVertexArrayObject | undefined
  private aPos = -1
  private uColorTexture: WebGLUniformLocation | null = null
  private uDepthTexture: WebGLUniformLocation | null = null
  private uDepthMode: WebGLUniformLocation | null = null
  private uHasDepthTexture: WebGLUniformLocation | null = null
  private readonly depthMode: TextureLayerDepthMode
  private readonly depthTextureProvider: TextureProvider | undefined

  constructor(
    private readonly colorTextureProvider: TextureProvider,
    options: TextureLayerOptions = {},
  ) {
    this.id = options.id ?? 'texture-layer'
    this.renderingMode = '3d'
    this.depthMode = options.depthMode ?? 'texture'
    this.depthTextureProvider = options.depthTexture
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map

    this.program = createProgram(gl, DEPTH_VERTEX_SHADER, DEPTH_FRAGMENT_SHADER)
    this.aPos = gl.getAttribLocation(this.program, 'a_pos')
    this.uColorTexture = gl.getUniformLocation(this.program, 'u_color_texture')
    this.uDepthTexture = gl.getUniformLocation(this.program, 'u_depth_texture')
    this.uDepthMode = gl.getUniformLocation(this.program, 'u_depth_mode')
    this.uHasDepthTexture = gl.getUniformLocation(this.program, 'u_has_depth_texture')

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
    if (!this.program || !this.vertexBuffer) {
      this.map.triggerRepaint()
      return
    }

    console.log('rtl ', this.map.painter.getDepthModeFor3D().range);

    const colorTexture = this.getTexture(this.colorTextureProvider)
    if (!colorTexture) {
      this.map.triggerRepaint()
      return
    }
    const depthTexture = this.depthTextureProvider
      ? this.getTexture(this.depthTextureProvider)
      : undefined

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

    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, colorTexture)
    gl.uniform1i(this.uColorTexture, 0)

    gl.activeTexture(gl.TEXTURE1)
    gl.bindTexture(gl.TEXTURE_2D, depthTexture ?? null)
    gl.uniform1i(this.uDepthTexture, 1)
    gl.uniform1i(this.uHasDepthTexture, depthTexture ? 1 : 0)

    gl.uniform1i(this.uDepthMode, depthModeToUniform(this.depthMode))
    gl.drawArrays(gl.TRIANGLES, 0, 6)

    this.map.triggerRepaint()
  }

  onRemove(_map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    if (this.vertexArray && isWebGL2(gl)) {
      gl.deleteVertexArray(this.vertexArray)
      this.vertexArray = undefined
    }
    if (this.vertexBuffer) {
      gl.deleteBuffer(this.vertexBuffer)
      this.vertexBuffer = undefined
    }
    if (this.program) {
      gl.deleteProgram(this.program)
      this.program = undefined
    }
  }

  private getTexture(provider: TextureProvider) {
    return typeof provider === 'function' ? provider() : provider
  }
}

function depthModeToUniform(mode: TextureLayerDepthMode): 0 | 1 | 2 {
  switch (mode) {
    case 'front':
      return 1
    case 'back':
      return 2
    case 'texture':
    default:
      return 0
  }
}

function isWebGL2(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
): gl is WebGL2RenderingContext {
  return 'createVertexArray' in gl
}

function createProgram(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
  vertexShaderSource: string,
  fragmentShaderSource: string,
) {
  const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource)
  const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource)
  const program = gl.createProgram()
  if (!program) throw new Error('Failed to create texture layer program')

  gl.attachShader(program, vertexShader)
  gl.attachShader(program, fragmentShader)
  gl.linkProgram(program)
  gl.deleteShader(vertexShader)
  gl.deleteShader(fragmentShader)

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(program) || 'Unknown texture layer program error'
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
  if (!shader) throw new Error('Failed to create texture layer shader')

  gl.shaderSource(shader, source)
  gl.compileShader(shader)

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader) || 'Unknown texture layer shader error'
    gl.deleteShader(shader)
    throw new Error(info)
  }

  return shader
}
