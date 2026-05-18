import { onBeforeUnmount, toValue, watchEffect, type WatchSource } from 'vue'
import type { FeatureCollection } from 'geojson'
import type { GeoJSONSource, Map as MapLibreMap } from 'maplibre-gl'

const HIGHLIGHT_SOURCE_ID = 'highlight'
const HIGHLIGHT_LAYER_ID = 'highlight'

export function useHighlightLayer({
  data,
  visible,
}: {
  data: WatchSource<FeatureCollection>
  visible: WatchSource<boolean>
}) {
  const onCleanupCallbacks: (() => void)[] = []

  const cleanup = () => {
    onCleanupCallbacks.splice(0).forEach((callback) => callback())
  }

  const register = (map: MapLibreMap) => {
    cleanup()

    map.addSource(HIGHLIGHT_SOURCE_ID, {
      type: 'geojson',
      data: toValue(data),
    })

    onCleanupCallbacks.push(
      watchEffect(() => {
        map.getSource<GeoJSONSource>(HIGHLIGHT_SOURCE_ID)?.setData(toValue(data))
      }).stop,
    )

    map.addLayer({
      id: HIGHLIGHT_LAYER_ID,
      source: HIGHLIGHT_SOURCE_ID,
      type: 'circle',
      paint: {
        'circle-radius': 25,
        'circle-color': 'transparent',
        'circle-stroke-color': '#1d87bf',
        'circle-stroke-opacity': 0.75,
        'circle-stroke-width': 3,
      },
    })

    onCleanupCallbacks.push(
      watchEffect(() => {
        map.setLayoutProperty(
          HIGHLIGHT_LAYER_ID,
          'visibility',
          toValue(visible) ? 'visible' : 'none',
        )
      }).stop,
    )
  }

  onBeforeUnmount(cleanup)

  return {
    register,
  }
}
