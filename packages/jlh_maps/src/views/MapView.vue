<template>
  <div style="position: absolute; left: 0; right: 0; top: 0; bottom: 0">
    <canvas class="hidden" id="water-render" style="position: absolute; inset: 0"></canvas>

    <mgl-map :map-style="tilejsonUrl" :center="[13.35203105083487, 52.499757263332086]" :zoom="14">
      <mgl-custom-control position="top-right">
        <button @click="slideoverOpen = SlideoverTab.Settings">
          <span class="maplibregl-ctrl-icon"></span>
        </button>
      </mgl-custom-control>
    </mgl-map>

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
import { MglMap, useMap } from '@indoorequal/vue-maplibre-gl'
import { computed, ref, watchEffect } from 'vue'
import { GeoJSONSource, GeolocateControl, GlobeControl, NavigationControl } from 'maplibre-gl'
import { center } from '@turf/turf'
import type { FeatureCollection } from 'geojson'
import { TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL } from '@/external/endpoints.ts'
import MapDetails from '@/components/MapDetails.vue'
import MapSettings from '@/components/MapSettings.vue'
import { DynWaterLayer } from '@/components/dyn-water-layer.ts'
import { useMapSelection } from '@/composables/maplibre.ts'
import { watchDefinedOnce } from '@/composables/helper.ts'

const mapInstance = useMap()

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

watchDefinedOnce(
  () => mapInstance.map,
  (map) => {
    map.addControl(new GlobeControl())
    map.addControl(new NavigationControl())
    map.addControl(new GeolocateControl({}))

    const onLoaded = () => {
      // 3D Buildings Layer

      map.addLayer(
        {
          id: '3d-buildings',
          source: 'openmaptiles',
          'source-layer': 'building',
          type: 'fill-extrusion',
          minzoom: 15,
          filter: ['!=', ['get', 'hide_3d'], true],
          layout: {
            visibility: 'none',
          },
          paint: {
            'fill-extrusion-color': [
              'interpolate',
              ['linear'],
              ['get', 'render_height'],
              0,
              'hsl(26, 12%, 82%)',
              400,
              'hsl(26, 15%, 82%)',
            ],
            'fill-extrusion-height': [
              'interpolate',
              ['linear'],
              ['zoom'],
              15,
              0,
              16,
              ['get', 'render_height'],
            ],
            'fill-extrusion-base': [
              'case',
              ['>=', ['get', 'zoom'], 16],
              ['get', 'render_min_height'],
              0,
            ],
          },
        },
        'Water labels',
      )

      map.on('pitch', () => {
        const visible = map.getPitch() > 20
        map.setLayoutProperty('3d-buildings', 'visibility', visible ? 'visible' : 'none')
      })

      // Dyn Water Layer

      const dynWaterLayer = new DynWaterLayer(map.getLayer('Water')!)
      map.addLayer(dynWaterLayer, 'Landcover patterns')
      map.setLayoutProperty(dynWaterLayer.id, 'visibility', 'visible')

      map.on('zoom', () => {
        const visible = map.getZoom() >= 14
        map.setLayoutProperty(dynWaterLayer.id, 'visibility', visible ? 'visible' : 'none')
      })

      // Highlight Layer

      map.addSource('highlight', {
        type: 'geojson',
        data: highlightGeoJsonData.value,
      })

      watchEffect(() => {
        map.getSource<GeoJSONSource>('highlight')?.setData(highlightGeoJsonData.value)
      })

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

      //

      selectableLayers.value = map.getLayersOrder()
      //.filter((layer) => map.getLayer(layer)?.type === 'symbol')
    }

    if (map.loaded()) {
      onLoaded()
    } else {
      map.on('load', onLoaded)
    }
  },
)
</script>

<style lang="css">
@import 'maplibre-gl/dist/maplibre-gl.css';
</style>
