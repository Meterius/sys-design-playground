<template>
  <div style="position: absolute; left: 0; right: 0; top: 0; bottom: 0">
    <mgl-map
      :map-style="tilejsonUrl"
      :center="[13.35203105083487, 52.499757263332086]"
      :zoom="14"
    >
    </mgl-map>

    <USlideover
      side="left"
      :modal="false"
      :overlay="false"
      :open="selection.length === 1"
      @update:open="
        (value) => {
          if (!value) {
            selection.splice(0)
          }
        }
      "
    >
      <template #content>
        <h2 class="p-4">Placeholder</h2>
        <UDivider />
        <UPlaceholder></UPlaceholder>
      </template>
    </USlideover>
  </div>
</template>

<script setup lang="ts">
import { MglMap, useMap } from '@indoorequal/vue-maplibre-gl'
import { computed, onMounted, reactive, watch, watchEffect } from 'vue'
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

const mapInstance = useMap()

const tilejsonUrl = TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL.toString();
console.debug('Using TileJson URL: ', tilejsonUrl);

const selection = reactive<
  {
    coords: LngLat
    feature: GeoJSONFeature
  }[]
>([])

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
