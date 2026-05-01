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
import { computed, onMounted, reactive, ref, watch, watchEffect } from 'vue'
import {
  GeoJSONFeature,
  GeoJSONSource,
  GeolocateControl,
  GlobeControl,
  LngLat,
  NavigationControl,
} from 'maplibre-gl'
import { center } from '@turf/turf'
import type { FeatureCollection } from 'geojson'
import { TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL } from '@/external/endpoints.ts'
import { extractOsmIdFromOmtFeatureId, type OsmId } from '@/external/osm.ts'
import MapDetails from '@/components/MapDetails.vue'
import MapSettings from '@/components/MapSettings.vue'
import { DynWaterLayer } from '@/components/dyn-water-layer.ts'

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
      selection.splice(0)
      break

    case SlideoverTab.Settings:
      break
  }

  slideoverOpen.value = null
}

const selection = reactive<
  {
    osm_id?: OsmId
    coords: LngLat
    feature: GeoJSONFeature
  }[]
>([])

watchEffect(() => {
  if (selection.length === 1) {
    slideoverOpen.value = SlideoverTab.Details
  } else if (selection.length !== 1 && slideoverOpen.value === SlideoverTab.Details) {
    slideoverOpen.value = null
  }
})

const highlightGeoJsonData = computed(
  (): FeatureCollection => ({
    type: 'FeatureCollection',
    features: selection.map((item) => center(item.feature.geometry)),
  }),
)

onMounted(() => {
  watch(
    mapInstance,
    (val, prev) => {
      if (val.map !== undefined && prev?.map === undefined) {
        const map = val.map

        map.addControl(new GlobeControl())
        map.addControl(new NavigationControl())
        map.addControl(new GeolocateControl({}))

        map.on('load', () => {
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

          const dynWaterLayer = new DynWaterLayer()
          map.addLayer(dynWaterLayer, 'Landcover patterns')
          map.setLayoutProperty(dynWaterLayer.id, 'visibility', 'visible')

          map.on('zoom', () => {
            const visible = map.getZoom() >= 14
            map.setLayoutProperty(dynWaterLayer.id, 'visibility', visible ? 'visible' : 'none')
          })

          map.on('pitch', () => {
            const visible = map.getPitch() > 20
            map.setLayoutProperty('3d-buildings', 'visibility', visible ? 'visible' : 'none')
          })

          map.on(
            'click',
            map.getStyle().layers.map((l) => l.id),
            (e) => {
              const feature = e.features?.find((f) => f.layer.type === 'symbol')

              if (feature) {
                console.log(feature, feature.layer, e)
                selection.splice(0)
                selection.push({
                  coords:
                    feature.geometry.type === 'Point'
                      ? new LngLat(
                          feature.geometry.coordinates[0] ?? 0,
                          feature.geometry.coordinates[1] ?? 0,
                        )
                      : e.lngLat,
                  feature,
                  osm_id:
                    typeof feature.id === 'number'
                      ? (extractOsmIdFromOmtFeatureId(feature.id) ?? undefined)
                      : undefined,
                })
              } else {
                selection.splice(0)
              }
            },
          )
        })
      }
    },
    { immediate: true },
  )
})
</script>

<style lang="css">
@import 'maplibre-gl/dist/maplibre-gl.css';
</style>
