import { onBeforeUnmount, onMounted, reactive, shallowRef, watch } from 'vue'
import {
  MapViewSettings as MapViewSettingsBevy,
  mount,
  resize_external_targets,
  set_map_view_settings,
  tick,
  unmount,
} from 'jlh_maps_app'

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

export function useBevy(bevyCanvasSelector: string, textureCanvasSelector: string) {
  const instanceId = bevyCanvasSelector

  const renderTexture = shallowRef<BevyRenderTexture>()
  const depthRenderTexture = shallowRef<BevyRenderTexture>()
  const depthTexture = shallowRef<WebGLTexture | null>()

  let resizeObserver: ResizeObserver | undefined
  let resizeHandler: (() => void) | undefined

  const onBeforeUnmountCallbacks: (() => void)[] = []

  const mapViewSettings = reactive(new MapViewSettingsBevy(false, true, true, true))

  onMounted(() => {
    const textureCanvas = document.querySelector<HTMLCanvasElement>(textureCanvasSelector)
    if (!textureCanvas) {
      throw new Error(`No texture canvas found for selector ${textureCanvasSelector}`)
    }

    const { height, width } = canvasRenderSize(textureCanvas)
    textureCanvas.width = width
    textureCanvas.height = height

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
      bevyCanvasSelector,
      texture.id,
      texture.width,
      texture.height,
      texture.framebuffer,
      r32fTexture.framebuffer,
    )

    onBeforeUnmountCallbacks.push(
      watch(
        mapViewSettings,
        (settings) => {
          set_map_view_settings(
            instanceId,
            new MapViewSettingsBevy(
              settings.enable_window_cameras,
              settings.enable_buildings,
              settings.enable_waters,
              settings.enable_shadows,
            ),
          )
        },
        { deep: true, immediate: true },
      ).stop,
    )

    resizeHandler = () => {
      const { height: nextHeight, width: nextWidth } = canvasRenderSize(textureCanvas)
      if (nextWidth === texture.width && nextHeight === texture.height) {
        return
      }

      textureCanvas.width = nextWidth
      textureCanvas.height = nextHeight
      texture.resize(nextWidth, nextHeight)
      r32fTexture.resize(nextWidth, nextHeight)
      resize_external_targets(instanceId, nextWidth, nextHeight)
    }

    resizeObserver = new ResizeObserver(resizeHandler)
    resizeObserver.observe(textureCanvas)
    window.addEventListener('resize', resizeHandler)
  })

  onBeforeUnmount(() => {
    resizeObserver?.disconnect()
    if (resizeHandler) {
      window.removeEventListener('resize', resizeHandler)
    }
    onBeforeUnmountCallbacks.splice(0).forEach((callback) => callback())
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
    mapViewSettings,
    tick: () => tick(instanceId),
  }
}

function canvasRenderSize(canvas: HTMLCanvasElement) {
  return {
    width: Math.max(1, Math.round(canvas.clientWidth * devicePixelRatio)),
    height: Math.max(1, Math.round(canvas.clientHeight * devicePixelRatio)),
  }
}
