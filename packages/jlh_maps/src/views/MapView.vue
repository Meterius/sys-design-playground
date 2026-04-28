<template>
  <div style="position: absolute; left: 0; right: 0; top: 0; bottom: 0">
    <mgl-map :map-style="mapStyleUrl" />
  </div>
</template>

<script setup lang="ts">
import {MglMap, useMap, MglGeolocateControl, MglNavigationControl} from '@indoorequal/vue-maplibre-gl'
import {watch} from "vue";
import {GeolocateControl, GlobeControl, NavigationControl} from "maplibre-gl";

const mapInstance = useMap();

const mapStyleUrl = import.meta.env.VITE_OTM_TILESERVER_TILEJSON_URL;
console.log(`Map style URL: ${mapStyleUrl}`);

const unwatch = watch(mapInstance, (val) => {
  if (val.map !== undefined) {
    unwatch();
    const map = val.map;

    map.addControl(new GlobeControl());
    map.addControl(new NavigationControl());
    map.addControl(new GeolocateControl({}));
  }
});
</script>

<style lang="css">
@import 'maplibre-gl/dist/maplibre-gl.css';
</style>
