import { onBeforeUnmount, onMounted, shallowRef } from 'vue'
import { mount, resize_external_targets, unmount } from 'jlh_maps_app'

interface BevyRenderTexture {
  id: number
  width: number
  height: number
  texture: WebGLTexture
  depthTexture: WebGLTexture | null
  framebuffer: WebGLFramebuffer
  resize: (width: number, height: number) => void
  dispose: () => void
}

declare global {
  interface Window {
    virtualWebGL?: {
      createRenderTexture?: (options?: { width?: number; height?: number }) => BevyRenderTexture
      createR32fRenderTexture?: (options?: { width?: number; height?: number }) => BevyRenderTexture
      createRenderTextureForCanvas?: (
        canvasOrSelector: HTMLCanvasElement | string,
        options?: { width?: number; height?: number },
      ) => BevyRenderTexture
      getRenderTexture?: (id: number) => BevyRenderTexture | undefined
    }
  }
}

export function useBevy(canvasSelector: string) {
  const instanceId = canvasSelector
  const renderTexture = shallowRef<BevyRenderTexture>()
  const depthRenderTexture = shallowRef<BevyRenderTexture>()
  const depthTexture = shallowRef<WebGLTexture | null>()
  let resizeObserver: ResizeObserver | undefined
  let resizeHandler: (() => void) | undefined

  onMounted(() => {
    const canvas = document.querySelector<HTMLCanvasElement>(canvasSelector)
    if (!canvas) {
      throw new Error(`No Bevy canvas found for selector ${canvasSelector}`)
    }

    const { height, width } = canvasRenderSize(canvas)
    canvas.width = width
    canvas.height = height

    const texture = window.virtualWebGL?.createRenderTexture?.({ width, height })
    const r32fTexture = window.virtualWebGL?.createR32fRenderTexture?.({ width, height })
    if (!texture) {
      throw new Error('virtualWebGL.createRenderTexture is not available')
    }
    if (!r32fTexture) {
      throw new Error('virtualWebGL.createR32fRenderTexture is not available')
    }

    renderTexture.value = texture
    depthRenderTexture.value = r32fTexture
    depthTexture.value = r32fTexture.texture
    mount(
      canvasSelector,
      texture.id,
      texture.width,
      texture.height,
      texture.framebuffer,
      r32fTexture.framebuffer,
    )

    resizeHandler = () => {
      const { height: nextHeight, width: nextWidth } = canvasRenderSize(canvas)
      if (nextWidth === texture.width && nextHeight === texture.height) {
        return
      }

      canvas.width = nextWidth
      canvas.height = nextHeight
      texture.resize(nextWidth, nextHeight)
      r32fTexture.resize(nextWidth, nextHeight)
      resize_external_targets(instanceId, nextWidth, nextHeight)
    }

    resizeObserver = new ResizeObserver(resizeHandler)
    resizeObserver.observe(canvas)
    window.addEventListener('resize', resizeHandler)
  })

  onBeforeUnmount(() => {
    resizeObserver?.disconnect()
    if (resizeHandler) {
      window.removeEventListener('resize', resizeHandler)
    }
    resizeObserver = undefined
    resizeHandler = undefined
    unmount(instanceId)
    renderTexture.value?.dispose()
    depthRenderTexture.value?.dispose()
    renderTexture.value = undefined
    depthRenderTexture.value = undefined
    depthTexture.value = undefined
  })

  return {
    instanceId,
    renderTexture,
    depthTexture,
  }
}

function canvasRenderSize(canvas: HTMLCanvasElement) {
  return {
    width: Math.max(1, Math.round(canvas.clientWidth * devicePixelRatio)),
    height: Math.max(1, Math.round(canvas.clientHeight * devicePixelRatio)),
  }
}
