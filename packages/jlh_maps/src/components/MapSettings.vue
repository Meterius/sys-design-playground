<template>
  <div class="grid overflow-auto overflow-x-hidden w-full">
    <div class="row p-4">
      <h1>Map Settings</h1>
    </div>
    <div class="row">
      <USeparator />
      <div class="row p-4">
        <h5 class="pb-2">Layers</h5>
        <UTree
          ref="layerTree"
          :nested="false"
          :unmount-on-hide="false"
          :items="layerItems"
          @select="$event.preventDefault()"
          class="border border-default rounded-md w-100 max-h-[400px] overflow-auto"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { MapLibreMap } from 'maplibre-gl'
import { shallowRef, useTemplateRef, watch } from 'vue'
import type { TreeItem } from '@nuxt/ui'
import { useSortable } from '@vueuse/integrations/useSortable'

const props = defineProps<{
  map: MapLibreMap
}>()

const layerItems = shallowRef<TreeItem[]>(
  props.map.getLayersOrder().map((layer) => ({
    layer,
    label: layer,
    icon: 'i-vscode-icons-file-type-maplibre',
  })),
)

watch(layerItems, () => {
  layerItems.value.forEach((item) => {
    props.map.moveLayer(item.layer)
  })
})

const layerTree = useTemplateRef<HTMLElement>('layerTree')

useSortable(layerTree, layerItems, {
  animation: 150,
  ghostClass: 'opacity-50',
})
</script>

<style scoped></style>
