<template>
  <div class="grid overflow-auto overflow-x-hidden w-full">
    <div class="row p-4">
      <h1>Map Settings</h1>
    </div>
    <div class="row">
      <USeparator />
      <div class="row p-4">
        <h5 class="pb-2">Debug</h5>
        <div class="grid gap-2">
          <label class="debug-toggle">
            <input v-model="showTileBoundaries" type="checkbox" />
            <span>Tile boundaries</span>
          </label>
          <label class="debug-toggle">
            <input v-model="showCollisionBoxes" type="checkbox" />
            <span>Collision boxes</span>
          </label>
          <label class="debug-toggle">
            <input v-model="showPadding" type="checkbox" />
            <span>Padding</span>
          </label>
        </div>
      </div>
    </div>
    <div class="row">
      <USeparator />
      <div class="row p-4">
        <h5 class="pb-2">Bevy</h5>
        <div class="grid gap-2">
          <label class="debug-toggle">
            <input v-model="enableBuildings" type="checkbox" />
            <span>Buildings</span>
          </label>
          <label class="debug-toggle">
            <input v-model="enableWaters" type="checkbox" />
            <span>Water</span>
          </label>
          <label class="debug-toggle">
            <input v-model="enableShadows" type="checkbox" />
            <span>Shadows</span>
          </label>
          <label class="debug-toggle">
            <input v-model="enableWindowCameras" type="checkbox" />
            <span>Debug canvas</span>
          </label>
        </div>
      </div>
    </div>
    <div class="row">
      <USeparator />
      <div class="row p-4">
        <h5 class="pb-2">Camera</h5>
        <div class="grid gap-2">
          <label class="debug-toggle">
            <input v-model="enableColorGrading" type="checkbox" />
            <span>Color grading</span>
          </label>
          <label class="debug-toggle">
            <input v-model="enableTonemapping" type="checkbox" />
            <span>Tonemapping</span>
          </label>
          <label class="debug-toggle">
            <input v-model="enableMsaa" type="checkbox" />
            <span>MSAA</span>
          </label>
          <label class="debug-toggle">
            <input v-model="enableSsao" type="checkbox" />
            <span>SSAO</span>
          </label>
          <label class="debug-toggle">
            <input v-model="enableTaa" type="checkbox" />
            <span>TAA</span>
          </label>
        </div>
      </div>
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
import { computed, shallowRef, useTemplateRef, watch } from 'vue'
import type { TreeItem } from '@nuxt/ui'
import { useSortable } from '@vueuse/integrations/useSortable'
import type {
  MapViewCameraSettings as MapViewCameraSettingsBevy,
  MapViewSettings as MapViewSettingsBevy,
} from 'jlh_maps_app'

const props = defineProps<{
  map: MapLibreMap
  bevySettings: MapViewSettingsBevy
  bevyCameraSettings: MapViewCameraSettingsBevy
}>()

const showTileBoundaries = computed({
  get: () => props.map.showTileBoundaries,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.map.showTileBoundaries = value
  },
})

const showCollisionBoxes = computed({
  get: () => props.map.showCollisionBoxes,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.map.showCollisionBoxes = value
  },
})

const showPadding = computed({
  get: () => props.map.showPadding,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.map.showPadding = value
  },
})

const enableBuildings = computed({
  get: () => props.bevySettings.enable_buildings,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevySettings.enable_buildings = value
  },
})

const enableWaters = computed({
  get: () => props.bevySettings.enable_waters,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevySettings.enable_waters = value
  },
})

const enableShadows = computed({
  get: () => props.bevySettings.enable_shadows,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevySettings.enable_shadows = value
  },
})

const enableWindowCameras = computed({
  get: () => props.bevySettings.enable_window_cameras,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevySettings.enable_window_cameras = value
  },
})

const enableColorGrading = computed({
  get: () => props.bevyCameraSettings.enable_color_grading,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevyCameraSettings.enable_color_grading = value
  },
})

const enableTonemapping = computed({
  get: () => props.bevyCameraSettings.enable_tonemapping,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevyCameraSettings.enable_tonemapping = value
  },
})

const enableSsao = computed({
  get: () => props.bevyCameraSettings.enable_ssao,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevyCameraSettings.enable_ssao = value
  },
})

const enableTaa = computed({
  get: () => props.bevyCameraSettings.enable_taa,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevyCameraSettings.enable_taa = value
  },
})

const enableMsaa = computed({
  get: () => props.bevyCameraSettings.enable_msaa,
  set: (value: boolean) => {
    // eslint-disable-next-line vue/no-mutating-props
    props.bevyCameraSettings.enable_msaa = value
  },
})

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

<style scoped>
.debug-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: fit-content;
  cursor: pointer;
}

.debug-toggle input {
  cursor: pointer;
}
</style>
