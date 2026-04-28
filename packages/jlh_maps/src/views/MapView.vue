<script setup lang="ts">
import { MglMap } from '@indoorequal/vue-maplibre-gl'
</script>

<template>
  <div style="position: absolute; left: 0; right: 0; top: 0; bottom: 0">
    <mgl-map map-style="https://demotiles.maplibre.org/style.json" />
  </div>
</template>

<style lang="css">
@import 'maplibre-gl/dist/maplibre-gl.css';
</style>
