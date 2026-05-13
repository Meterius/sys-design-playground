import type { CustomLayerInterface, Map as MapLibreMap } from 'maplibre-gl'

export type OffscreenTransferMode =
  | 'main-2d-canvas'
  | 'main-gl-shader'
  | 'worker-2d-imagebitmap'
  | 'worker-webgl-imagebitmap'
  | 'worker-arraybuffer'
  | 'same-context-fbo'

interface OffscreenCanvasTestLayerOptions {
  id?: string
  mode?: OffscreenTransferMode
  logEveryFrames?: number
}

type WorkerFrame =
  | {
      type: 'imagebitmap-frame'
      bitmap: ImageBitmap
      drawStart: number
      drawEnd: number
      width: number
      height: number
      frameId: number
    }
  | {
      type: 'arraybuffer-frame'
      pixels: ArrayBuffer
      drawStart: number
      drawEnd: number
      width: number
      height: number
      frameId: number
    }

type PendingFrame =
  | {
      kind: 'imagebitmap'
      bitmap: ImageBitmap
      drawStart: number
      drawEnd: number
      receivedAt: number
      width: number
      height: number
      frameId: number
    }
  | {
      kind: 'arraybuffer'
      pixels: ArrayBuffer
      width: number
      height: number
      drawStart: number
      drawEnd: number
      receivedAt: number
      frameId: number
    }

interface UploadedWorkerFrameStats {
  uploadedFrameId: number
  workerDrawMs: number
  messageLatencyMs: number
  frameAgeMs: number
  receiveToUploadMs: number
}

export class OffscreenCanvasTestLayer implements CustomLayerInterface {
  id: string
  type = 'custom' as const
  renderingMode = '2d' as const

  private readonly mode: OffscreenTransferMode
  private readonly logEveryFrames: number
  private map!: MapLibreMap
  private canvas: OffscreenCanvas | undefined
  private ctx: OffscreenCanvasRenderingContext2D | undefined
  private worker: Worker | undefined
  private pendingFrame: PendingFrame | undefined
  private workerFrameInFlight = false
  private frameId = 0
  private requestedWorkerFrameId = 0
  private latestReceivedWorkerFrameId = -1
  private latestUploadedWorkerFrameId = -1
  private replacedPendingWorkerFrames = 0
  private staleWorkerFrames = 0

  private blitProgram: WebGLProgram | undefined
  private gradientProgram: WebGLProgram | undefined
  private vertexBuffer: WebGLBuffer | undefined
  private texture: WebGLTexture | undefined
  private fboTexture: WebGLTexture | undefined
  private framebuffer: WebGLFramebuffer | undefined
  private aPos = -1
  private aUv = -1
  private uTexture: WebGLUniformLocation | null = null
  private uHue: WebGLUniformLocation | null = null
  private textureWidth = 0
  private textureHeight = 0

  constructor(options: OffscreenCanvasTestLayerOptions = {}) {
    this.id = options.id ?? 'offscreen-canvas-test'
    this.mode = options.mode ?? 'worker-2d-imagebitmap'
    this.logEveryFrames = options.logEveryFrames ?? 120
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map
    this.blitProgram = createProgram(gl, BLIT_VERTEX_SHADER, BLIT_FRAGMENT_SHADER)
    this.aPos = gl.getAttribLocation(this.blitProgram, 'a_pos')
    this.aUv = gl.getAttribLocation(this.blitProgram, 'a_uv')
    this.uTexture = gl.getUniformLocation(this.blitProgram, 'u_texture')
    this.vertexBuffer = createFullscreenQuad(gl)

    if (this.mode === 'main-gl-shader' || this.mode === 'same-context-fbo') {
      this.gradientProgram = createProgram(gl, GRADIENT_VERTEX_SHADER, GRADIENT_FRAGMENT_SHADER)
      this.uHue = gl.getUniformLocation(this.gradientProgram, 'u_hue')
    }

    if (this.mode === 'main-2d-canvas') {
      this.createMainCanvas()
      this.texture = createTexture(gl)
    } else if (this.mode === 'same-context-fbo') {
      this.texture = createTexture(gl)
      this.fboTexture = createTexture(gl)
      this.framebuffer = gl.createFramebuffer() ?? undefined
    } else if (this.mode.startsWith('worker-')) {
      this.texture = createTexture(gl)
      this.createWorker()
    }

    console.debug('OffscreenCanvas test layer initialized', {
      mode: this.mode,
      ...mapCanvasSize(map),
    })
  }

  render(gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    const frameStart = performance.now()
    const { width, height } = mapCanvasSize(this.map)
    const hue = frameStart * 0.04
    let uploadMs = 0
    let workerDrawMs = 0
    let messageLatencyMs = 0
    let frameAgeMs = 0
    let receiveToUploadMs = 0
    let uploadedFrameId: number | undefined

    if (this.mode === 'main-gl-shader') {
      this.drawShaderGradient(gl, null, width, height, hue)
      this.finishFrame(frameStart, { uploadMs: 0 })
      return
    }

    if (this.mode === 'same-context-fbo') {
      this.ensureSameContextFbo(gl, width, height)
      this.drawShaderGradient(gl, this.framebuffer ?? null, width, height, hue)
      const uploadStart = performance.now()
      gl.bindTexture(gl.TEXTURE_2D, this.texture ?? null)
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.framebuffer ?? null)
      if (this.textureWidth !== width || this.textureHeight !== height) {
        gl.copyTexImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 0, 0, width, height, 0)
        this.textureWidth = width
        this.textureHeight = height
      } else {
        gl.copyTexSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 0, 0, width, height)
      }
      uploadMs = performance.now() - uploadStart
    } else {
      if (this.mode === 'main-2d-canvas') {
        this.syncMainCanvasSize()
        if (this.canvas && this.ctx) {
          draw2dGradient(this.canvas, this.ctx, hue)
          const uploadStart = performance.now()
          this.uploadCanvas(gl, this.canvas)
          uploadMs = performance.now() - uploadStart
        }
      } else {
        this.requestWorkerFrame(width, height, hue)
        const frame = this.pendingFrame
        this.pendingFrame = undefined
        if (frame) {
          const workerFrameStats = this.workerFrameStats(frame)
          workerDrawMs = workerFrameStats.workerDrawMs
          messageLatencyMs = workerFrameStats.messageLatencyMs
          frameAgeMs = workerFrameStats.frameAgeMs
          receiveToUploadMs = workerFrameStats.receiveToUploadMs
          uploadedFrameId = workerFrameStats.uploadedFrameId
          const uploadStart = performance.now()
          if (frame.kind === 'imagebitmap') {
            this.uploadImageBitmap(gl, frame.bitmap)
            frame.bitmap.close()
          } else {
            this.uploadArrayBuffer(gl, frame.pixels, frame.width, frame.height)
          }
          uploadMs = performance.now() - uploadStart
        }
      }
    }

    if (this.texture) {
      this.drawTexture(gl)
    }
    this.finishFrame(frameStart, {
      uploadMs,
      workerDrawMs,
      messageLatencyMs,
      frameAgeMs,
      receiveToUploadMs,
      uploadedFrameId,
    })
  }

  onRemove(_map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.worker?.terminate()
    this.worker = undefined
    this.pendingFrame?.kind === 'imagebitmap' && this.pendingFrame.bitmap.close()
    this.pendingFrame = undefined

    if (this.framebuffer) gl.deleteFramebuffer(this.framebuffer)
    if (this.fboTexture) gl.deleteTexture(this.fboTexture)
    if (this.texture) gl.deleteTexture(this.texture)
    if (this.vertexBuffer) gl.deleteBuffer(this.vertexBuffer)
    if (this.blitProgram) gl.deleteProgram(this.blitProgram)
    if (this.gradientProgram) gl.deleteProgram(this.gradientProgram)

    this.framebuffer = undefined
    this.fboTexture = undefined
    this.texture = undefined
    this.vertexBuffer = undefined
    this.blitProgram = undefined
    this.gradientProgram = undefined
    this.canvas = undefined
    this.ctx = undefined
  }

  private createMainCanvas() {
    if (!('OffscreenCanvas' in window)) {
      console.warn('OffscreenCanvas is not available in this browser')
      return
    }
    const { width, height } = mapCanvasSize(this.map)
    const canvas = new OffscreenCanvas(width, height)
    const ctx = canvas.getContext('2d')
    if (!ctx) {
      console.warn('Could not create OffscreenCanvas 2D context')
      return
    }
    this.canvas = canvas
    this.ctx = ctx
  }

  private createWorker() {
    this.worker = new Worker(new URL('./offscreen-canvas-test-worker.ts', import.meta.url), {
      type: 'module',
    })
    this.worker.onmessage = (event: MessageEvent<WorkerFrame>) => {
      this.workerFrameInFlight = false
      const now = performance.now()
      if (this.pendingFrame?.kind === 'imagebitmap') {
        this.pendingFrame.bitmap.close()
      }
      if (this.pendingFrame) {
        this.replacedPendingWorkerFrames += 1
      }
      if (event.data.frameId < this.latestReceivedWorkerFrameId) {
        this.staleWorkerFrames += 1
      }
      this.latestReceivedWorkerFrameId = Math.max(this.latestReceivedWorkerFrameId, event.data.frameId)
      if (event.data.type === 'imagebitmap-frame') {
        this.pendingFrame = {
          kind: 'imagebitmap',
          bitmap: event.data.bitmap,
          drawStart: event.data.drawStart,
          drawEnd: event.data.drawEnd,
          receivedAt: now,
          width: event.data.width,
          height: event.data.height,
          frameId: event.data.frameId,
        }
      } else {
        this.pendingFrame = {
          kind: 'arraybuffer',
          pixels: event.data.pixels,
          width: event.data.width,
          height: event.data.height,
          drawStart: event.data.drawStart,
          drawEnd: event.data.drawEnd,
          receivedAt: now,
          frameId: event.data.frameId,
        }
      }
      this.map.triggerRepaint()
    }
  }

  private requestWorkerFrame(width: number, height: number, hue: number) {
    if (!this.worker || this.workerFrameInFlight) return
    this.workerFrameInFlight = true
    const frameId = this.requestedWorkerFrameId++
    this.worker.postMessage({
      type: 'render',
      mode: this.mode,
      width,
      height,
      hue,
      frameId,
    })
  }

  private syncMainCanvasSize() {
    if (!this.canvas) return
    const { width, height } = mapCanvasSize(this.map)
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width
      this.canvas.height = height
      this.textureWidth = 0
      this.textureHeight = 0
    }
  }

  private ensureSameContextFbo(gl: WebGLRenderingContext | WebGL2RenderingContext, width: number, height: number) {
    if (!this.fboTexture || !this.framebuffer) return
    if (this.textureWidth === width && this.textureHeight === height) return

    gl.bindTexture(gl.TEXTURE_2D, this.fboTexture)
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null)
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.framebuffer)
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.fboTexture, 0)
    gl.bindFramebuffer(gl.FRAMEBUFFER, null)
    this.textureWidth = 0
    this.textureHeight = 0
  }

  private uploadCanvas(gl: WebGLRenderingContext | WebGL2RenderingContext, canvas: OffscreenCanvas) {
    gl.bindTexture(gl.TEXTURE_2D, this.texture ?? null)
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true)
    if (this.textureWidth !== canvas.width || this.textureHeight !== canvas.height) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, canvas)
      this.textureWidth = canvas.width
      this.textureHeight = canvas.height
    } else {
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, gl.RGBA, gl.UNSIGNED_BYTE, canvas)
    }
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false)
  }

  private uploadImageBitmap(gl: WebGLRenderingContext | WebGL2RenderingContext, bitmap: ImageBitmap) {
    gl.bindTexture(gl.TEXTURE_2D, this.texture ?? null)
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true)
    if (this.textureWidth !== bitmap.width || this.textureHeight !== bitmap.height) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, bitmap)
      this.textureWidth = bitmap.width
      this.textureHeight = bitmap.height
    } else {
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, gl.RGBA, gl.UNSIGNED_BYTE, bitmap)
    }
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false)
  }

  private uploadArrayBuffer(
    gl: WebGLRenderingContext | WebGL2RenderingContext,
    pixels: ArrayBuffer,
    width: number,
    height: number,
  ) {
    gl.bindTexture(gl.TEXTURE_2D, this.texture ?? null)
    const data = new Uint8Array(pixels)
    if (this.textureWidth !== width || this.textureHeight !== height) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, data)
      this.textureWidth = width
      this.textureHeight = height
    } else {
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, data)
    }
  }

  private drawShaderGradient(
    gl: WebGLRenderingContext | WebGL2RenderingContext,
    target: WebGLFramebuffer | null,
    width: number,
    height: number,
    hue: number,
  ) {
    if (!this.gradientProgram || !this.vertexBuffer) return
    gl.bindFramebuffer(gl.FRAMEBUFFER, target)
    gl.viewport(0, 0, width, height)
    gl.disable(gl.DEPTH_TEST)
    gl.depthMask(false)
    gl.useProgram(this.gradientProgram)
    gl.uniform1f(this.uHue, hue)
    bindFullscreenQuad(gl, this.vertexBuffer, gl.getAttribLocation(this.gradientProgram, 'a_pos'), -1)
    gl.drawArrays(gl.TRIANGLES, 0, 6)
    gl.bindFramebuffer(gl.FRAMEBUFFER, null)
  }

  private drawTexture(gl: WebGLRenderingContext | WebGL2RenderingContext) {
    if (!this.blitProgram || !this.vertexBuffer || !this.texture) return
    const { width, height } = mapCanvasSize(this.map)
    gl.bindFramebuffer(gl.FRAMEBUFFER, null)
    gl.viewport(0, 0, width, height)
    gl.enable(gl.BLEND)
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)
    gl.disable(gl.DEPTH_TEST)
    gl.depthMask(false)
    gl.useProgram(this.blitProgram)
    bindFullscreenQuad(gl, this.vertexBuffer, this.aPos, this.aUv)
    gl.activeTexture(gl.TEXTURE0)
    gl.bindTexture(gl.TEXTURE_2D, this.texture)
    gl.uniform1i(this.uTexture, 0)
    gl.drawArrays(gl.TRIANGLES, 0, 6)
  }

  private workerFrameStats(frame: PendingFrame): UploadedWorkerFrameStats {
    const now = performance.now()
    this.latestUploadedWorkerFrameId = frame.frameId
    return {
      uploadedFrameId: frame.frameId,
      workerDrawMs: frame.drawEnd - frame.drawStart,
      messageLatencyMs: frame.receivedAt - frame.drawEnd,
      frameAgeMs: now - frame.drawEnd,
      receiveToUploadMs: now - frame.receivedAt,
    }
  }

  private finishFrame(
    frameStart: number,
    stats: {
      uploadMs: number
      workerDrawMs?: number
      messageLatencyMs?: number
      frameAgeMs?: number
      receiveToUploadMs?: number
      uploadedFrameId?: number
    },
  ) {
    this.frameId += 1
    if (this.frameId % this.logEveryFrames === 0) {
      console.debug('OffscreenCanvas test frame', {
        mode: this.mode,
        frame: this.frameId,
        uploadMs: stats.uploadMs,
        workerDrawMs: stats.workerDrawMs ?? 0,
        messageLatencyMs: stats.messageLatencyMs ?? 0,
        frameAgeMs: stats.frameAgeMs ?? 0,
        receiveToUploadMs: stats.receiveToUploadMs ?? 0,
        requestedWorkerFrameId: this.requestedWorkerFrameId - 1,
        latestReceivedWorkerFrameId: this.latestReceivedWorkerFrameId,
        latestUploadedWorkerFrameId: this.latestUploadedWorkerFrameId,
        uploadedFrameId: stats.uploadedFrameId,
        replacedPendingWorkerFrames: this.replacedPendingWorkerFrames,
        staleWorkerFrames: this.staleWorkerFrames,
        pendingWorkerFrameId: this.pendingFrame?.frameId,
        workerFrameInFlight: this.workerFrameInFlight,
        totalMainRenderMs: performance.now() - frameStart,
      })
    }
    this.map.triggerRepaint()
  }
}

function createTexture(gl: WebGLRenderingContext | WebGL2RenderingContext) {
  const texture = gl.createTexture() ?? undefined
  gl.bindTexture(gl.TEXTURE_2D, texture ?? null)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
  gl.bindTexture(gl.TEXTURE_2D, null)
  return texture
}

function createFullscreenQuad(gl: WebGLRenderingContext | WebGL2RenderingContext) {
  const buffer = gl.createBuffer() ?? undefined
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer ?? null)
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([
      -1.0, -1.0, 0.0, 0.0,
      1.0, -1.0, 1.0, 0.0,
      -1.0, 1.0, 0.0, 1.0,
      -1.0, 1.0, 0.0, 1.0,
      1.0, -1.0, 1.0, 0.0,
      1.0, 1.0, 1.0, 1.0,
    ]),
    gl.STATIC_DRAW,
  )
  gl.bindBuffer(gl.ARRAY_BUFFER, null)
  return buffer
}

function bindFullscreenQuad(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
  buffer: WebGLBuffer,
  positionLocation: number,
  uvLocation: number,
) {
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
  gl.enableVertexAttribArray(positionLocation)
  gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 16, 0)
  if (uvLocation >= 0) {
    gl.enableVertexAttribArray(uvLocation)
    gl.vertexAttribPointer(uvLocation, 2, gl.FLOAT, false, 16, 8)
  }
}

function draw2dGradient(canvas: OffscreenCanvas, ctx: OffscreenCanvasRenderingContext2D, hue: number) {
  const h0 = hue % 360
  const h1 = (h0 + 80) % 360

  ctx.clearRect(0, 0, canvas.width, canvas.height)
  const gradient = ctx.createLinearGradient(0, 0, canvas.width, 0)
  gradient.addColorStop(0.0, `hsla(${h0}, 95%, 55%, 0.0)`)
  gradient.addColorStop(0.5, `hsla(${h0}, 95%, 55%, 0.65)`)
  gradient.addColorStop(1.0, `hsla(${h1}, 95%, 78%, 1.0)`)
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, canvas.width, canvas.height)
}

function mapCanvasSize(map: MapLibreMap) {
  const canvas = map.getCanvas()
  return {
    width: Math.max(1, canvas.width),
    height: Math.max(1, canvas.height),
  }
}

const BLIT_VERTEX_SHADER = `
attribute vec2 a_pos;
attribute vec2 a_uv;
varying vec2 v_uv;

void main() {
  v_uv = a_uv;
  gl_Position = vec4(a_pos, 0.0, 1.0);
}
`

const BLIT_FRAGMENT_SHADER = `
precision mediump float;
varying vec2 v_uv;
uniform sampler2D u_texture;

void main() {
  gl_FragColor = texture2D(u_texture, v_uv);
}
`

const GRADIENT_VERTEX_SHADER = `
attribute vec2 a_pos;
varying vec2 v_uv;

void main() {
  v_uv = a_pos * 0.5 + 0.5;
  gl_Position = vec4(a_pos, 0.0, 1.0);
}
`

const GRADIENT_FRAGMENT_SHADER = `
precision mediump float;
varying vec2 v_uv;
uniform float u_hue;

vec3 hsv2rgb(vec3 c) {
  vec4 k = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
  return c.z * mix(k.xxx, clamp(p - k.xxx, 0.0, 1.0), c.y);
}

void main() {
  float h0 = mod(u_hue, 360.0) / 360.0;
  float h1 = mod(u_hue + 80.0, 360.0) / 360.0;
  vec3 c0 = hsv2rgb(vec3(h0, 0.95, 0.55));
  vec3 c1 = hsv2rgb(vec3(h1, 0.95, 0.78));
  float alpha = smoothstep(0.0, 1.0, v_uv.x);
  gl_FragColor = vec4(mix(c0, c1, v_uv.x), alpha);
}
`

function createProgram(
  gl: WebGLRenderingContext | WebGL2RenderingContext,
  vertexShaderSource: string,
  fragmentShaderSource: string,
) {
  const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource)
  const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource)
  const program = gl.createProgram()
  if (!program) throw new Error('Failed to create offscreen canvas test program')

  gl.attachShader(program, vertexShader)
  gl.attachShader(program, fragmentShader)
  gl.linkProgram(program)
  gl.deleteShader(vertexShader)
  gl.deleteShader(fragmentShader)

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(program) || 'Unknown offscreen canvas test program error'
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
  if (!shader) throw new Error('Failed to create offscreen canvas test shader')

  gl.shaderSource(shader, source)
  gl.compileShader(shader)

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader) || 'Unknown offscreen canvas test shader error'
    gl.deleteShader(shader)
    throw new Error(info)
  }

  return shader
}
