type WorkerRenderMode = 'worker-2d-imagebitmap' | 'worker-webgl-imagebitmap' | 'worker-arraybuffer'

interface RenderRequest {
  type: 'render'
  mode: WorkerRenderMode
  width: number
  height: number
  hue: number
  frameId: number
}

let canvas2d: OffscreenCanvas | undefined
let ctx2d: OffscreenCanvasRenderingContext2D | undefined
let canvasGl: OffscreenCanvas | undefined
let gl: WebGLRenderingContext | undefined
let glProgram: WebGLProgram | undefined
let glBuffer: WebGLBuffer | undefined
let glHue: WebGLUniformLocation | null = null
let glPosition = -1

self.onmessage = (event: MessageEvent<RenderRequest>) => {
  if (event.data.type !== 'render') return

  const drawStart = performance.now()
  if (event.data.mode === 'worker-webgl-imagebitmap') {
    const canvas = renderWebglFrame(event.data.width, event.data.height, event.data.hue)
    const bitmap = canvas.transferToImageBitmap()
    const drawEnd = performance.now()
    self.postMessage(
      {
        type: 'imagebitmap-frame',
        bitmap,
        drawStart,
        drawEnd,
        width: canvas.width,
        height: canvas.height,
        frameId: event.data.frameId,
      },
      [bitmap],
    )
    return
  }

  const { canvas, ctx } = ensure2dCanvas(event.data.width, event.data.height)
  draw2dGradient(canvas, ctx, event.data.hue)
  const drawEnd = performance.now()

  if (event.data.mode === 'worker-arraybuffer') {
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height)
    self.postMessage(
      {
        type: 'arraybuffer-frame',
        pixels: imageData.data.buffer,
        drawStart,
        drawEnd,
        width: canvas.width,
        height: canvas.height,
        frameId: event.data.frameId,
      },
      [imageData.data.buffer],
    )
    return
  }

  const bitmap = canvas.transferToImageBitmap()
  self.postMessage(
    {
      type: 'imagebitmap-frame',
      bitmap,
      drawStart,
      drawEnd,
      width: canvas.width,
      height: canvas.height,
      frameId: event.data.frameId,
    },
    [bitmap],
  )
}

function ensure2dCanvas(width: number, height: number) {
  if (!canvas2d) {
    canvas2d = new OffscreenCanvas(width, height)
    ctx2d = canvas2d.getContext('2d') ?? undefined
    if (!ctx2d) throw new Error('Could not create worker OffscreenCanvas 2D context')
  }
  if (canvas2d.width !== width || canvas2d.height !== height) {
    canvas2d.width = width
    canvas2d.height = height
  }
  return { canvas: canvas2d, ctx: ctx2d! }
}

function renderWebglFrame(width: number, height: number, hue: number) {
  if (!canvasGl) {
    canvasGl = new OffscreenCanvas(width, height)
    gl = canvasGl.getContext('webgl', { premultipliedAlpha: true, alpha: true }) ?? undefined
    if (!gl) throw new Error('Could not create worker OffscreenCanvas WebGL context')
    glProgram = createProgram(gl, GRADIENT_VERTEX_SHADER, GRADIENT_FRAGMENT_SHADER)
    glPosition = gl.getAttribLocation(glProgram, 'a_pos')
    glHue = gl.getUniformLocation(glProgram, 'u_hue')
    glBuffer = gl.createBuffer() ?? undefined
    gl.bindBuffer(gl.ARRAY_BUFFER, glBuffer ?? null)
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
      gl.STATIC_DRAW,
    )
  }
  if (canvasGl.width !== width || canvasGl.height !== height) {
    canvasGl.width = width
    canvasGl.height = height
  }
  gl!.viewport(0, 0, width, height)
  gl!.disable(gl!.DEPTH_TEST)
  gl!.useProgram(glProgram!)
  gl!.uniform1f(glHue, hue)
  gl!.bindBuffer(gl!.ARRAY_BUFFER, glBuffer!)
  gl!.enableVertexAttribArray(glPosition)
  gl!.vertexAttribPointer(glPosition, 2, gl!.FLOAT, false, 0, 0)
  gl!.drawArrays(gl!.TRIANGLES, 0, 6)
  gl!.flush()
  return canvasGl
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

function createProgram(gl: WebGLRenderingContext, vertexShaderSource: string, fragmentShaderSource: string) {
  const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource)
  const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource)
  const program = gl.createProgram()
  if (!program) throw new Error('Failed to create worker offscreen canvas program')

  gl.attachShader(program, vertexShader)
  gl.attachShader(program, fragmentShader)
  gl.linkProgram(program)
  gl.deleteShader(vertexShader)
  gl.deleteShader(fragmentShader)

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(program) || 'Unknown worker offscreen canvas program error'
    gl.deleteProgram(program)
    throw new Error(info)
  }

  return program
}

function createShader(gl: WebGLRenderingContext, type: number, source: string) {
  const shader = gl.createShader(type)
  if (!shader) throw new Error('Failed to create worker offscreen canvas shader')

  gl.shaderSource(shader, source)
  gl.compileShader(shader)

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader) || 'Unknown worker offscreen canvas shader error'
    gl.deleteShader(shader)
    throw new Error(info)
  }

  return shader
}

export {}
