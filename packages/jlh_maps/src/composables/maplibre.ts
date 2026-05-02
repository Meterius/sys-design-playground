import { useMap } from '@indoorequal/vue-maplibre-gl'
import { extractOsmIdFromOmtFeatureId, type OsmId } from '@/utils/osm.ts'
import {
  GeoJSONFeature,
  LngLat,
  type MapLayerMouseEvent,
  type MapMouseEvent,
  type Subscription,
} from 'maplibre-gl'
import { onUnmounted, ref, watch, type WatchSource } from 'vue'
import { get } from '@vueuse/core'

const CLICK_LAYER_SYNC_BUFFER_MS = 50

export interface SelectionItem {
  osm_id?: OsmId
  coords: LngLat
  feature: GeoJSONFeature
}

export function useMapSelection(options: {
  key?: symbol | string
  targetLayers: WatchSource<string[]>
}) {
  const mapInstance = useMap(options.key)

  const selection = ref<SelectionItem[]>([])
  let lastTargetLayerClick: MapMouseEvent | undefined
  let clearSelectionTimeout: ReturnType<typeof setTimeout> | undefined

  const clearSelection = () => {
    selection.value.splice(0)
  }

  const clicksMatch = (click: MapMouseEvent, targetLayerClick: MapMouseEvent | undefined) => {
    if (!targetLayerClick) {
      return false
    }

    return (
      click.originalEvent === targetLayerClick.originalEvent ||
      (click.originalEvent.timeStamp === targetLayerClick.originalEvent.timeStamp &&
        click.point.x === targetLayerClick.point.x &&
        click.point.y === targetLayerClick.point.y)
    )
  }

  const makeOnClick = (targetLayers: string[]) => (e: MapLayerMouseEvent) => {
    console.log('Click Event', e, e.features)
    lastTargetLayerClick = e

    const features = e.features?.filter((f) => targetLayers.includes(f.layer.id)) ?? []
    const selectedFeature = features[0]

    if (selectedFeature) {
      clearSelection()
      selection.value.push({
        coords:
          selectedFeature.geometry.type === 'Point'
            ? new LngLat(
                selectedFeature.geometry.coordinates[0] ?? 0,
                selectedFeature.geometry.coordinates[1] ?? 0,
              )
            : e.lngLat,
        feature: selectedFeature,
        osm_id:
          typeof selectedFeature.id === 'number'
            ? (extractOsmIdFromOmtFeatureId(selectedFeature.id) ?? undefined)
            : undefined,
      })
    } else {
      clearSelection()
    }
  }

  const onMapClick = (e: MapMouseEvent) => {
    if (clearSelectionTimeout) {
      clearTimeout(clearSelectionTimeout)
      clearSelectionTimeout = undefined
    }

    clearSelectionTimeout = setTimeout(() => {
      clearSelectionTimeout = undefined

      if (!clicksMatch(e, lastTargetLayerClick)) {
        clearSelection()
      }
    }, CLICK_LAYER_SYNC_BUFFER_MS)
  }

  let onClickSubscription: Subscription | undefined
  let onMapClickSubscription: Subscription | undefined
  watch(
    () => ({
      map: mapInstance.map,
      targetLayers: [...get(options.targetLayers)],
    }),
    ({ map, targetLayers }) => {
      onClickSubscription?.unsubscribe()
      onMapClickSubscription?.unsubscribe()
      onClickSubscription = undefined
      onMapClickSubscription = undefined
      lastTargetLayerClick = undefined

      if (map) {
        onClickSubscription = map.on('click', targetLayers, makeOnClick(targetLayers))
        onMapClickSubscription = map.on('click', onMapClick)
      }
    },
    { immediate: true },
  )

  onUnmounted(() => {
    if (clearSelectionTimeout) {
      clearTimeout(clearSelectionTimeout)
      clearSelectionTimeout = undefined
    }

    onClickSubscription?.unsubscribe()
    onMapClickSubscription?.unsubscribe()
    onClickSubscription = undefined
    onMapClickSubscription = undefined
  })

  return {
    selection,
  }
}
