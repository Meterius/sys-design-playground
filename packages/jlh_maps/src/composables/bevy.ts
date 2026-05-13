import { onBeforeUnmount, onMounted, reactive, shallowRef, watch } from 'vue'
import {
  MapViewSettings as MapViewSettingsBevy,
  forward_cursor_entered,
  forward_cursor_left,
  forward_cursor_moved,
  forward_focus,
  forward_keyboard_input,
  forward_mouse_button,
  forward_mouse_wheel,
  mount,
  resize,
  set_map_view_settings,
  tick,
  unmount,
} from 'jlh_maps_app'

export function useBevy(debugCanvasSelector: string, textureCanvasSelector: string) {
  const instanceId = debugCanvasSelector

  const debugOffscreenCanvas = shallowRef<OffscreenCanvas | null>(null)
  const textureOffscreenCanvas = shallowRef<OffscreenCanvas | null>(null)
  let resizeObserver: ResizeObserver | undefined
  let resizeHandler: (() => void) | undefined

  const onBeforeUnmountCallbacks: (() => void)[] = []

  const mapViewSettings = reactive(new MapViewSettingsBevy(false, true, true, true))

  onMounted(() => {
    const textureCanvas = document.querySelector<HTMLCanvasElement>(textureCanvasSelector)
    if (!textureCanvas) {
      throw new Error(`No texture canvas found for selector ${textureCanvasSelector}`)
    }

    const debugCanvas = document.querySelector<HTMLCanvasElement>(debugCanvasSelector)
    if (!debugCanvas) {
      throw new Error(`No Bevy canvas found for selector ${debugCanvasSelector}`)
    }
    debugCanvas.tabIndex = 0

    const initialSize = canvasRenderSize(textureCanvas)
    debugCanvas.width = initialSize.width
    debugCanvas.height = initialSize.height
    debugOffscreenCanvas.value = debugCanvas.transferControlToOffscreen()
    textureOffscreenCanvas.value = new OffscreenCanvas(initialSize.width, initialSize.height)

    mount(instanceId, debugOffscreenCanvas.value, textureOffscreenCanvas.value)
    resize(instanceId, initialSize.width, initialSize.height, initialSize.width, initialSize.height, devicePixelRatio)

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
      const { width, height } = canvasRenderSize(textureCanvas)
      if (debugOffscreenCanvas.value) {
        debugOffscreenCanvas.value.width = width
        debugOffscreenCanvas.value.height = height
      }
      if (textureOffscreenCanvas.value) {
        textureOffscreenCanvas.value.width = width
        textureOffscreenCanvas.value.height = height
      }
      resize(instanceId, width, height, width, height, devicePixelRatio)
    }

    const removeEventForwarding = forwardDebugCanvasEvents(debugCanvas, instanceId)
    onBeforeUnmountCallbacks.push(removeEventForwarding)

    resizeObserver = new ResizeObserver(resizeHandler)
    resizeObserver.observe(textureCanvas)
    resizeObserver.observe(debugCanvas)
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
    debugOffscreenCanvas.value = null
    textureOffscreenCanvas.value = null
    unmount(instanceId)
  })

  return {
    instanceId,
    mapViewSettings,
    tick: () => {
      tick(instanceId)
    },
    mapTextureOffscreenCanvas: textureOffscreenCanvas,
  }
}

function forwardDebugCanvasEvents(canvas: HTMLCanvasElement, instanceId: string) {
  const canvasPosition = (event: MouseEvent | PointerEvent) => {
    const rect = canvas.getBoundingClientRect()
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top,
    }
  }

  const onPointerEnter = () => {
    forward_cursor_entered(instanceId)
  }
  const onPointerLeave = () => {
    forward_cursor_left(instanceId)
  }
  const onPointerMove = (event: PointerEvent) => {
    const position = canvasPosition(event)
    forward_cursor_moved(instanceId, position.x, position.y, event.movementX, event.movementY)
  }
  const onPointerDown = (event: PointerEvent) => {
    canvas.focus()
    canvas.setPointerCapture(event.pointerId)
    forward_mouse_button(instanceId, event.button, true)
  }
  const onPointerUp = (event: PointerEvent) => {
    if (canvas.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId)
    }
    forward_mouse_button(instanceId, event.button, false)
  }
  const onWheel = (event: WheelEvent) => {
    event.preventDefault()
    forward_mouse_wheel(instanceId, event.deltaX, event.deltaY, event.deltaMode)
  }
  const onFocus = () => {
    forward_focus(instanceId, true)
  }
  const onBlur = () => {
    forward_focus(instanceId, false)
  }
  const onKeyDown = (event: KeyboardEvent) => {
    forward_keyboard_input(instanceId, event.code, event.key, true, event.repeat)
  }
  const onKeyUp = (event: KeyboardEvent) => {
    forward_keyboard_input(instanceId, event.code, event.key, false, event.repeat)
  }

  canvas.addEventListener('pointerenter', onPointerEnter)
  canvas.addEventListener('pointerleave', onPointerLeave)
  canvas.addEventListener('pointermove', onPointerMove)
  canvas.addEventListener('pointerdown', onPointerDown)
  canvas.addEventListener('pointerup', onPointerUp)
  canvas.addEventListener('wheel', onWheel, { passive: false })
  canvas.addEventListener('focus', onFocus)
  canvas.addEventListener('blur', onBlur)
  canvas.addEventListener('keydown', onKeyDown)
  canvas.addEventListener('keyup', onKeyUp)

  return () => {
    canvas.removeEventListener('pointerenter', onPointerEnter)
    canvas.removeEventListener('pointerleave', onPointerLeave)
    canvas.removeEventListener('pointermove', onPointerMove)
    canvas.removeEventListener('pointerdown', onPointerDown)
    canvas.removeEventListener('pointerup', onPointerUp)
    canvas.removeEventListener('wheel', onWheel)
    canvas.removeEventListener('focus', onFocus)
    canvas.removeEventListener('blur', onBlur)
    canvas.removeEventListener('keydown', onKeyDown)
    canvas.removeEventListener('keyup', onKeyUp)
  }
}

function canvasRenderSize(canvas: HTMLCanvasElement) {
  return {
    width: Math.max(1, Math.round(canvas.clientWidth * devicePixelRatio)),
    height: Math.max(1, Math.round(canvas.clientHeight * devicePixelRatio)),
  }
}
