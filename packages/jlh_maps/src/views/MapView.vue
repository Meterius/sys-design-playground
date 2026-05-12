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

    <USlideover
      side="left"
      :modal="false"
      :overlay="false"
      :open="slideoverOpen !== null"
      @update:open="
        (value: boolean) => {
          if (!value) onSlideoverClose()
        }
      "
    >
      <template #content>
        <map-details
          v-if="slideoverOpen === SlideoverTab.Details"
          :osm_id="selection[0]?.osm_id"
          :feature="selection[0]?.feature"
        />

        <map-settings
          v-if="slideoverOpen === SlideoverTab.Settings && mapInstance.map"
          :map="mapInstance.map"
        />
      </template>
    </USlideover>
  </div>
</template>

<script setup lang="ts">
import { MglMap } from '@indoorequal/vue-maplibre-gl'
import { computed, onWatcherCleanup, ref, watch, watchEffect } from 'vue'
import { GeoJSONSource, GeolocateControl, GlobeControl, NavigationControl } from 'maplibre-gl'
import { center } from '@turf/turf'
import type { FeatureCollection } from 'geojson'
import {
  TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL,
  TILESERVER_RASTER_SEN2_TILEJSON_URL,
} from '@/external/endpoints.ts'
import MapDetails from '@/components/MapDetails.vue'
import MapSettings from '@/components/MapSettings.vue'
import { DynWaterLayer } from '../maplibre-layers/dyn-water-layer.ts'
import { TreeMeshLayer } from '../maplibre-layers/tree-mesh-layer.ts'
import { makeUniqueMapKey, useMapExtended, useMapSelection } from '@/composables/maplibre.ts'
import { watchDefinedOnce } from '@/composables/helper.ts'
import { useMaplibreGlJsIntegration } from '@/composables/bevy-maplibre-integration.ts'
import { useBevy } from '@/composables/bevy.ts'
import { BevyLayer } from '../maplibre-layers/bevy-layer.ts'

const mapKey = makeUniqueMapKey()

const bevyCanvasId = `bevy-canvas-${mapKey}`

const { depthTexture, instanceId, renderTexture, tick, enableWindowCameras } = useBevy(
  `#${bevyCanvasId}`,
  '.maplibregl-canvas',
)

const { mapInstance, loaded, zoom } = useMapExtended(mapKey)

const { syncOnRender } = useMaplibreGlJsIntegration(() => instanceId, mapKey, {
  featureSourceLayers: [{ sourceId: 'openmaptiles', sourceLayer: 'building' }],
})

const tilejsonUrl = TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL.toString()
console.debug('Using TileJson URL: ', tilejsonUrl)

enum SlideoverTab {
  Details,
  Settings,
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

const showBevyCanvas = ref(false)

watch(
  showBevyCanvas,
  (value) => {
    enableWindowCameras.value = value
  },
  { immediate: true },
)

const terrainEnabled = ref(false)

const useRasterOnly = false
const useRaster = false

const enableTrees = false
const enableDynWater = false

watchDefinedOnce(
  () => mapInstance.map,
  (map) => {
    map.addControl(new GlobeControl())
    map.addControl(new NavigationControl())
    map.addControl(new GeolocateControl({}))

    map.setMaxPitch(85)
  },
)

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

    // 3D Buildings Layer

    // map.addLayer(
    //   {
    //     id: '3d-buildings',
    //     source: 'openmaptiles',
    //     'source-layer': 'building',
    //     type: 'fill-extrusion',
    //     minzoom: 15,
    //     layout: {
    //       visibility: 'none',
    //     },
    //     paint: {
    //       'fill-extrusion-color': [
    //         'interpolate',
    //         ['linear'],
    //         ['get', 'render_height'],
    //         0,
    //         'hsl(26, 12%, 82%)',
    //         400,
    //         'hsl(26, 15%, 82%)',
    //       ],
    //       'fill-extrusion-height': ['get', 'render_height'],
    //       'fill-extrusion-base': ['get', 'render_min_height'],
    //       'fill-extrusion-vertical-gradient': true,
    //     },
    //   },
    //   'Water labels',
    // )
    //
    // ;['Oneway path', 'Oneway', 'Oneway opposite'].forEach((layerId) => {
    //   const layer = map.getStyle().layers.find((l) => l.id === layerId)!
    //   map.removeLayer(layerId)
    //   map.addLayer(layer, '3d-buildings')
    // })

    // onCleanupCallbacks.push(
    //   watchEffect(() => {
    //     const visible = (pitch.value > 20 || terrainEnabled.value) && !useRaster
    //     map.setLayoutProperty('3d-buildings', 'visibility', visible ? 'visible' : 'none')
    //   }).stop,
    // )

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

    // Dyn Water Layer

    if (enableDynWater) {
      const dynWaterLayer = new DynWaterLayer(map.getLayer('Water')!)
      map.addLayer(dynWaterLayer, 'Landcover patterns')
      map.setLayoutProperty(dynWaterLayer.id, 'visibility', useRaster ? 'none' : 'visible')

      onCleanupCallbacks.push(
        watch(zoom, (value) => {
          const visible = value >= 14 && !useRaster
          map.setLayoutProperty(dynWaterLayer.id, 'visibility', visible ? 'visible' : 'none')
        }).stop,
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

    map.addLayer(
      new BevyLayer(() => renderTexture.value?.texture, {
        id: 'bevy-texture',
        depthMode: 'texture',
        depthTexture: () => depthTexture.value,
        tick: () => {
          syncOnRender()
          tick()
        },
      }),
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

    selectableLayers.value = map
      .getLayersOrder()
      .filter((layer) => map.getLayer(layer)?.type === 'symbol')

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
