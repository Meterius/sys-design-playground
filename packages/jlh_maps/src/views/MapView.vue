<template>
  <div style="position: absolute; left: 0; right: 0; top: 0; bottom: 0">
    <div
      :style="`position: absolute; width: 100%; height: ${showBevyCanvas ? '50%' : '100%'}; top: 0`"
    >
      <mgl-map
        :map-key="mapKey"
        :map-style="tilejsonUrl"
        :center="[13.35203105083487, 52.499757263332086]"
        :zoom="14"
        :canvas-context-attributes="{ antialias: true }"
        @map:contextmenu="onMapContextMenu"
      >
        <mgl-custom-control position="top-right">
          <button
            class="map-custom-control"
            type="button"
            title="Map settings"
            aria-label="Map settings"
            @click="slideoverOpen = SlideoverTab.Settings"
          >
            <UIcon
              name="material-symbols:settings-outline-rounded"
              class="size-6"
              style="margin: auto"
            />
          </button>
        </mgl-custom-control>

        <mgl-custom-control position="top-right">
          <button
            class="map-custom-control"
            type="button"
            title="Navigation"
            aria-label="Navigation"
            @click="slideoverOpen = SlideoverTab.Directions"
          >
            <UIcon
              name="material-symbols:signpost-outline-rounded"
              class="size-6"
              style="margin: auto"
            />
          </button>
        </mgl-custom-control>

        <mgl-custom-control position="top-right">
          <button
            class="map-custom-control"
            type="button"
            title="Toggle terrain"
            aria-label="Toggle terrain"
            :aria-pressed="terrainEnabled"
            @click="terrainEnabled = !terrainEnabled"
          >
            <UIcon
              name="material-symbols:elevation-outline-rounded"
              :class="['size-6', ...[terrainEnabled ? ['text-secondary'] : []]]"
              style="margin: auto"
            />
          </button>
        </mgl-custom-control>

        <mgl-custom-control position="top-right">
          <button
            class="map-custom-control"
            type="button"
            title="Show bevy"
            aria-label="Show bevy"
            :aria-pressed="showBevyCanvas"
            @click="showBevyCanvas = !showBevyCanvas"
          >
            <UIcon
              name="material-symbols:bug-report-outline-rounded"
              :class="['size-6', ...[showBevyCanvas ? ['text-secondary'] : []]]"
              style="margin: auto"
            />
          </button>
        </mgl-custom-control>
      </mgl-map>
    </div>

    <div
      v-show="showBevyCanvas"
      :style="`position: absolute; width: ${showBevyCanvas ? '100%' : '10px'}; height: ${showBevyCanvas ? '50%' : '1px'}; bottom: 0`"
    >
      <canvas
        :id="bevyCanvasId"
        style="position: absolute; inset: 0; height: 100%; width: 100%"
      ></canvas>
    </div>

    <UContextMenu :items="contextMenuItems" :modal="false">
      <div
        ref="mapContextMenuTarget"
        class="h-full w-full absolute"
        style="pointer-events: none"
        @contextmenu="console.log"
      ></div>
    </UContextMenu>

    <USlideover
      side="left"
      :modal="false"
      :overlay="false"
      :dismissible="false"
      :open="slideoverOpen !== null"
      @update:open="
        (value: boolean) => {
          if (!value) onSlideoverClose()
        }
      "
    >
      <template #content>
        <div class="relative h-full">
          <UButton
            class="absolute right-3 top-3 z-10 rounded-full cursor-pointer"
            icon="material-symbols:close-rounded"
            title="Close"
            aria-label="Close"
            variant="ghost"
            color="neutral"
            size="md"
            square
            :ui="{ leadingIcon: 'size-6' }"
            @click="onSlideoverClose"
          />

          <map-directions
            v-show="slideoverOpen === SlideoverTab.Directions"
            v-model:stops="directionStops"
          />

          <map-details
            v-show="slideoverOpen === SlideoverTab.Details"
            :osm_id="selection[0]?.osm_id"
            :feature="selection[0]?.feature"
          />

          <map-settings
            v-if="mapInstance.map"
            v-show="slideoverOpen === SlideoverTab.Settings"
            :map="mapInstance.map"
            :bevy-settings="mapViewSettings"
            :bevy-camera-settings="mapViewCameraSettings"
          />
        </div>
      </template>
    </USlideover>
  </div>
</template>

<script setup lang="ts">
import { MglMap } from '@indoorequal/vue-maplibre-gl'
import { computed, onWatcherCleanup, ref, shallowRef, watch, watchEffect } from 'vue'
import {
  GeoJSONSource,
  GeolocateControl,
  GlobeControl,
  type Map as MaplibreMap,
  NavigationControl,
  type MapMouseEvent,
} from 'maplibre-gl'
import { center } from '@turf/turf'
import mapPinIconSvg from 'lucide-static/icons/map-pin.svg?raw'
import type { FeatureCollection, Point } from 'geojson'
import {
  TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL,
  TILESERVER_RASTER_SEN2_TILEJSON_URL,
} from '@/external/endpoints.ts'
import MapDetails from '@/components/MapDetails.vue'
import MapSettings from '@/components/MapSettings.vue'
import { TreeMeshLayer } from '../maplibre-layers/tree-mesh-layer.ts'
import { makeUniqueMapKey, useMapExtended, useMapSelection } from '@/composables/maplibre.ts'
import { watchDefinedOnce } from '@/composables/helper.ts'
import { useMaplibreGlJsIntegration } from '@/composables/bevy-maplibre-integration.ts'
import { useBevy } from '@/composables/bevy.ts'
import { BevyLayer } from '../maplibre-layers/bevy-layer.ts'
import MapDirections from '@/components/MapDirections.vue'
import { GeoLocationType, type GeoLocation } from '@/components/types.ts'
import type { ContextMenuItem } from '@nuxt/ui'
import { svgToImage } from '@/utils/svg-to-image.ts'

const mapKey = makeUniqueMapKey()

const bevyCanvasId = `bevy-canvas-${mapKey}`

const { instanceId, mapViewSettings, mapViewCameraSettings, tick, mapTextureOffscreenCanvas } =
  useBevy(`#${bevyCanvasId}`, '.maplibregl-canvas')

const { mapInstance, loaded, zoom } = useMapExtended(mapKey)

const { syncOnRender } = useMaplibreGlJsIntegration(() => instanceId, mapKey, {
  featureSourceLayers: [
    { sourceId: 'openmaptiles', sourceLayer: 'building' },
    { sourceId: 'openmaptiles', sourceLayer: 'water' },
  ],
})

const tilejsonUrl = TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL.toString()
console.debug('Using TileJson URL: ', tilejsonUrl)

// Context Menu

const mapContextMenuTarget = ref<HTMLElement | null>(null)
const contextMenuLocation = shallowRef<GeoLocation | null>(null)

type MglMapMouseEvent = {
  type: string
  event: MapMouseEvent
}

const onMapContextMenu = ({ event }: MglMapMouseEvent) => {
  event.preventDefault()
  event.originalEvent.preventDefault()

  contextMenuLocation.value = {
    type: GeoLocationType.Coords,
    coords: {
      lat: event.lngLat.lat,
      lng: event.lngLat.lng,
    },
  }

  mapContextMenuTarget.value?.dispatchEvent(
    new MouseEvent('contextmenu', {
      bubbles: true,
      cancelable: true,
      clientX: event.originalEvent.clientX,
      clientY: event.originalEvent.clientY,
    }),
  )
}

const setDirectionStop = (idx: number) => {
  if (!contextMenuLocation.value) return

  const stops = [...directionStops.value]
  stops[idx] = contextMenuLocation.value

  directionStops.value = stops
  slideoverOpen.value = SlideoverTab.Directions
}

const contextMenuCoordinateLabel = computed(() => {
  const location = contextMenuLocation.value
  if (!location) return 'No location selected'

  return `${location.coords.lat.toFixed(6)}, ${location.coords.lng.toFixed(6)}`
})

const contextMenuItems = computed((): ContextMenuItem[] => [
  {
    label: contextMenuCoordinateLabel.value,
    type: 'label',
    icon: 'material-symbols:location-on-outline-rounded',
  },
  {
    type: 'separator',
  },
  {
    label: 'Directions From Here',
    icon: 'material-symbols:line-end-circle-outline-rounded',
    ui: {
      itemLeadingIcon: '-rotate-90',
    } as unknown as ContextMenuItem['ui'],
    onSelect: () => setDirectionStop(0),
    disabled: directionStops.value.length < 1,
  },
  {
    label: 'Directions To Here',
    icon: 'material-symbols:line-end-circle-outline-rounded',
    ui: {
      itemLeadingIcon: 'rotate-90',
    } as unknown as ContextMenuItem['ui'],
    onSelect: () => setDirectionStop(directionStops.value.length - 1),
    disabled: directionStops.value.length < 2,
  },
])

// Slideover

enum SlideoverTab {
  Details,
  Settings,
  Directions,
}

const slideoverOpen = ref<SlideoverTab | null>(null)

const onSlideoverClose = () => {
  switch (slideoverOpen.value) {
    case SlideoverTab.Details:
      selection.value.splice(0)
      break

    case SlideoverTab.Settings:
      break
  }

  slideoverOpen.value = null
}

// Directions

const directionStops = shallowRef<(GeoLocation | null)[]>([null, null])

// Selection

const selectableLayers = ref<string[]>([])

const { selection } = useMapSelection({
  key: mapKey,
  targetLayers: selectableLayers,
})

watchEffect(() => {
  if (selection.value.length === 1) {
    slideoverOpen.value = SlideoverTab.Details
  } else if (selection.value.length !== 1 && slideoverOpen.value === SlideoverTab.Details) {
    slideoverOpen.value = null
  }
})

const highlightGeoJsonData = computed(
  (): FeatureCollection => ({
    type: 'FeatureCollection',
    features: selection.value.map((item) => center(item.feature.geometry)),
  }),
)

// Direction Stops Layer

type DirectionStopProperties = {
  label: string
  sortKey: number
}

const DIRECTION_STOPS_SOURCE_ID = 'direction-stops'
const DIRECTION_STOPS_SHADOW_LAYER_ID = 'direction-stops-shadow'
const DIRECTION_STOPS_LAYER_ID = 'direction-stops'
const DIRECTION_STOP_ICON_ID = 'lucide:map-pin'

const directionStopsGeoJsonData = computed(
  (): FeatureCollection<Point, DirectionStopProperties> => {
    const lastIdx = directionStops.value.length - 1

    return {
      type: 'FeatureCollection',
      features: directionStops.value.flatMap((stop, idx) => {
        if (!stop) return []

        const isStart = idx === 0
        const isEnd = idx === lastIdx

        return [
          {
            type: 'Feature',
            geometry: {
              type: 'Point',
              coordinates: [stop.coords.lng, stop.coords.lat],
            },
            properties: {
              label: isStart ? 'S' : isEnd ? 'E' : String(idx),
              sortKey: idx,
            },
          },
        ]
      }),
    }
  },
)

const registerDirectionStopImage = async (map: MaplibreMap) => {
  if (!map.hasImage(DIRECTION_STOP_ICON_ID)) {
    const { image } = await svgToImage(mapPinIconSvg, {
      width: 24,
      pixelRatio: 2,
      color: '#2563eb',
    })

    map.addImage(DIRECTION_STOP_ICON_ID, image, {
      pixelRatio: 2,
    })
  }
}

//

const showBevyCanvas = ref(false)

watch(
  showBevyCanvas,
  (value) => {
    mapViewSettings.enable_window_cameras = value
  },
  { immediate: true },
)

const terrainEnabled = ref(false)

const useRasterOnly = false
const useRaster = false

const enableTrees = false

// Controls

watchDefinedOnce(
  () => mapInstance.map,
  (map) => {
    map.addControl(new GlobeControl())
    map.addControl(new NavigationControl())
    map.addControl(new GeolocateControl({}))

    map.setMaxPitch(85)
  },
)

// Maplibre Setup

watchDefinedOnce(
  () => {
    if (!loaded.value) return undefined

    return mapInstance.map !== undefined ? { map: mapInstance.map } : undefined
  },
  ({ map }) => {
    const onCleanupCallbacks: (() => void)[] = []

    if (useRaster) {
      map.addSource('raster-sen2', {
        type: 'raster',
        url: TILESERVER_RASTER_SEN2_TILEJSON_URL.toString(),
      })

      map.addLayer(
        {
          id: 'raster-sen2-layer',
          type: 'raster',
          source: 'raster-sen2',
          paint: {
            'raster-brightness-min': 0.1,
            'raster-contrast': 0.2,
          },
        },
        'Residential',
      )

      map.getLayersOrder().forEach((layerId) => {
        if (layerId === 'raster-sen2-layer') return

        const layer = map.getLayer(layerId)!

        if (useRasterOnly) {
          layer.setLayoutProperty('visibility', 'none')
          return
        }

        switch (layer.type) {
          case 'symbol':
            map.setPaintProperty(layerId, 'icon-opacity', [
              'interpolate',
              ['linear'],
              ['zoom'],
              13,
              0.0,
              16,
              0.75,
            ])
            map.setPaintProperty(layerId, 'text-opacity', [
              'interpolate',
              ['linear'],
              ['zoom'],
              13,
              0.0,
              16,
              0.75,
            ])
            break

          case 'fill':
            map.setPaintProperty(
              layerId,
              'fill-outline-color',
              layer.getPaintProperty('fill-color'),
            )
            map.setPaintProperty(layerId, 'fill-color', 'transparent')
            map.setPaintProperty(layerId, 'fill-opacity', [
              'interpolate',
              ['linear'],
              ['zoom'],
              14,
              0,
              16,
              0.25,
            ])
            break

          case 'line':
            map.setPaintProperty(layerId, 'line-opacity', 0.05)
            break

          default:
            map.setLayoutProperty(layerId, 'visibility', 'none')
            break
        }
      })
    }

    // Sky / Terrain / Hillshade

    map.addSource('terrain', {
      type: 'raster-dem',
      url: 'https://tiles.mapterhorn.com/tilejson.json',
      maxzoom: 16,
    })

    map.addSource('hillshade', {
      type: 'raster-dem',
      url: 'https://tiles.mapterhorn.com/tilejson.json',
      maxzoom: 16,
    })

    map.setSky({
      'sky-color': '#199EF3',
      'sky-horizon-blend': 0.7,
      'horizon-color': 'rgb(236 248 251)',
      'horizon-fog-blend': 0.9,
      'fog-color': 'rgb(165 209 223 / 0.5)',
      'fog-ground-blend': 0.8,
      'atmosphere-blend': ['interpolate', ['linear'], ['zoom'], 0, 0.45, 7, 0.25, 10, 0],
    })

    onCleanupCallbacks.push(
      watch(
        zoom,
        (value) => {
          if (value < 10) {
            map.setLight({
              anchor: 'map',
              position: [1.5, 90, 80],
              intensity: 0.25,
            })
          } else {
            map.setLight({
              anchor: 'viewport',
              position: [1.15, 210, 30],
              intensity: 0.5,
            })
          }
        },
        { immediate: true },
      ).stop,
    )

    onCleanupCallbacks.push(
      watch(
        terrainEnabled,
        (enabled) => {
          if (enabled) {
            map.setTerrain({
              source: 'terrain',
              exaggeration: 1.0,
            })
          } else {
            map.setTerrain(null)
          }
        },
        { immediate: true },
      ).stop,
    )

    map.addLayer({
      id: 'hills',
      type: 'hillshade',
      source: 'hillshade',
      paint: {
        'hillshade-exaggeration': useRaster ? 0.4 : 0.5,
        'hillshade-shadow-color': useRaster ? 'rgb(0 0 0 / 0.8)' : 'rgb(71 59 36 / 0.84)',
        'hillshade-highlight-color': useRaster
          ? 'rgb(255 255 255 / 0.29)'
          : 'rgb(255 255 255 / 0.84)',
        'hillshade-method': useRaster ? 'igor' : 'combined',
      },
    })

    onCleanupCallbacks.push(
      watch(
        terrainEnabled,
        (enabled) => {
          if (enabled) {
            map.setLayoutProperty('hills', 'visibility', 'visible')
          } else {
            map.setLayoutProperty('hills', 'visibility', 'none')
          }
        },
        { immediate: true },
      ).stop,
    )

    // Bevy

    map.addLayer(
      new BevyLayer(mapTextureOffscreenCanvas, {
        id: 'bevy-texture',
        tick: () => {
          syncOnRender()
          tick()
        },
      }),
      'Water labels',
    )
    ;['Oneway path', 'Oneway', 'Oneway opposite'].forEach((layerId) => {
      const layer = map.getStyle().layers.find((l) => l.id === layerId)!
      map.removeLayer(layerId)
      map.addLayer(layer, 'bevy-texture')
    })

    // Tree Mesh Layer

    if (enableTrees) {
      const forestLayer = map.getLayer('Wood')!
      const treeMeshLayer = new TreeMeshLayer(forestLayer)
      map.addLayer(treeMeshLayer, 'Water labels')

      onCleanupCallbacks.push(
        watch(
          zoom,
          (value) => {
            const visible = value >= 14 && !useRaster
            map.setLayoutProperty(treeMeshLayer.id, 'visibility', visible ? 'visible' : 'none')
          },
          { immediate: true },
        ).stop,
      )
    }

    // Highlight Layer

    map.addSource('highlight', {
      type: 'geojson',
      data: highlightGeoJsonData.value,
    })

    onCleanupCallbacks.push(
      watchEffect(() => {
        map.getSource<GeoJSONSource>('highlight')?.setData(highlightGeoJsonData.value)
      }).stop,
    )

    map.addLayer({
      id: 'highlight',
      source: 'highlight',
      type: 'circle',
      paint: {
        'circle-radius': 25,
        'circle-color': 'transparent',
        'circle-stroke-color': '#1d87bf',
        'circle-stroke-opacity': 0.75,
        'circle-stroke-width': 3,
      },
    })

    // Direction Stops Layer

    void registerDirectionStopImage(map)

    map.addSource(DIRECTION_STOPS_SOURCE_ID, {
      type: 'geojson',
      data: directionStopsGeoJsonData.value,
    })

    onCleanupCallbacks.push(
      watchEffect(() => {
        map
          .getSource<GeoJSONSource>(DIRECTION_STOPS_SOURCE_ID)
          ?.setData(directionStopsGeoJsonData.value)
      }).stop,
    )

    map.addLayer({
      id: DIRECTION_STOPS_SHADOW_LAYER_ID,
      source: DIRECTION_STOPS_SOURCE_ID,
      type: 'circle',
      paint: {
        'circle-radius': 16,
        'circle-blur': 0.4,
        'circle-color': '#000000',
        'circle-opacity': 0.3,
        'circle-translate': [0, 0],
        'circle-translate-anchor': 'viewport',
        'circle-pitch-alignment': 'map',
      },
    })

    map.addLayer({
      id: DIRECTION_STOPS_LAYER_ID,
      source: DIRECTION_STOPS_SOURCE_ID,
      type: 'symbol',
      layout: {
        'icon-image': DIRECTION_STOP_ICON_ID,
        'icon-size': 1.5,
        'icon-anchor': 'bottom',
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
        'text-field': ['get', 'label'],
        'text-anchor': 'bottom',
        'text-offset': [0, -2.25],
        'text-size': 16,
        'text-allow-overlap': true,
        'text-ignore-placement': true,
        'symbol-sort-key': ['get', 'sortKey'],
      },
      paint: {
        'text-color': '#111827',
        'text-halo-color': '#ffffff',
        'text-halo-width': 2,
      },
    })

    selectableLayers.value = map
      .getLayersOrder()
      .filter(
        (layer) => map.getLayer(layer)?.type === 'symbol' && layer !== DIRECTION_STOPS_LAYER_ID,
      )

    // Clean-Up

    onWatcherCleanup(() => {
      onCleanupCallbacks.forEach((cleanup) => cleanup())
    })
  },
)
</script>

<style lang="css">
@import 'maplibre-gl/dist/maplibre-gl.css';

.maplibregl-canvas {
  background: #131d25;
}

.map-custom-control {
  width: 29px;
  height: 29px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 0;
  padding: 0;
  background: #fff;
  color: #333;
  cursor: pointer;
}

.map-custom-control:hover {
  background: #f2f2f2;
}

.map-custom-control:focus-visible {
  outline: 2px solid #2563eb;
  outline-offset: -2px;
}

.map-custom-control.is-active {
  background: #e0f2fe;
  color: #0369a1;
  box-shadow: inset 0 0 0 2px #0284c7;
}

.map-custom-control-icon {
  width: 17px;
  height: 17px;
  display: block;
}
</style>
