<template>
  <div class="grid overflow-auto overflow-x-hidden w-full">
    <div class="row p-4">
      <h1>{{ title }}</h1>
    </div>
    <div v-if="props.osm_id" class="row">
      <USeparator />
      <div class="row p-4">
        <h5 class="pb-2">OSM Tags</h5>
        <UTable
          sticky
          :data="tagTableData"
          :loading="loadingOsmData"
          :ui="{
            td: 'py-2',
            root: 'relative overflow-auto',
            base: 'overflow-clip',
            tbody: 'isolate',
          }"
          class="flex-1 border border-default rounded-md w-100 max-h-[400px]"
        ></UTable>
      </div>
    </div>
    <div class="row">
      <USeparator />
      <div class="row p-4">
        <h5 class="pb-2">Feature Properties</h5>
        <UTable
          sticky
          :data="tableData"
          :ui="{
            td: 'py-2',
            root: 'relative overflow-auto',
            base: 'overflow-clip',
            tbody: 'isolate',
          }"
          class="flex-1 border border-default rounded-md w-100 max-h-[400px]"
        ></UTable>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watchEffect } from 'vue'
import type { GeoJSONFeature } from 'maplibre-gl'
import { computedAsync } from '@vueuse/core'
import { getOsmData } from '@/external/endpoints.ts'
import type { OsmId } from '@/external/osm.ts'

const props = defineProps<{
  osm_id?: OsmId
  feature?: GeoJSONFeature
}>()

const title = computed(() => {
  return props.feature?.properties?.name ?? 'Location Details'
})

const loadingOsmData = ref(false)

const osmData = computedAsync(
  async () => (props.osm_id ? getOsmData(props.osm_id) : null),
  null,
  loadingOsmData,
)

watchEffect(() => {
  console.log('Fetched osm data: {}', osmData.value)
})

const tableData = computed(() => {
  return Object.entries(props.feature?.properties ?? {}).map(([key, value]) => ({ key, value }))
})

const tagTableData = computed(() => {
  return Object.entries(osmData.value?.tags ?? {}).map(([key, value]) => ({ key, value }))
})
</script>

<style scoped></style>
