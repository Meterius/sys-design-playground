<script setup lang="ts">
  import { initialize } from "jlh-sys-design-playground";
  import {onMounted} from "vue";

  onMounted(() => {
    initialize();
  })
</script>

<template>
  <div style="z-index: 50; position: absolute; left: 0; right: 0; bottom: 0; top: 0;">
    <canvas id="render-canvas" style="height: 100%; width: 100%; margin: 0;" autofocus></canvas>
  </div>
</template>

<style scoped></style>
