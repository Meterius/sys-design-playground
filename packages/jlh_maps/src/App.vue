<template>
  <UApp>
    <UMain>
      <RouterView />
    </UMain>
  </UApp>
</template>

<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<style>
@import 'tailwindcss';
@import '@nuxt/ui';
</style>
