import { onBeforeUnmount, onMounted } from 'vue'
import { mount, unmount } from 'jlh_maps_app'

export function useBevy(canvasSelector: string) {
  const instanceId = canvasSelector

  onMounted(() => {
    mount(canvasSelector)
  })

  onBeforeUnmount(() => {
    unmount(instanceId)
  })

  return {
    instanceId,
  }
}
