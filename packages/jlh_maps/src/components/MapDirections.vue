<template>
  <div class="grid overflow-auto overflow-x-hidden w-full">
    <div class="p-4">
      <h1 class="font-semibold">Directions</h1>
    </div>
    <div class="grid min-h-0">
      <USeparator />
      <div class="grid content-start gap-3 p-4">
        <div
          v-for="(stop, idx) in stops"
          :key="idx"
          :data-has-stop="stop !== null"
          class="grid grid-cols-[2rem_minmax(0,1fr)] items-center gap-3"
        >
          <UIcon :name="getStopIconName(idx)" :class="['mx-auto size-6', getStopIconClass(idx)]" />
          <div class="min-w-0">
            <UInput
              class="w-full"
              readonly
              :model-value="formatStop(stop)"
              :placeholder="getStopPlaceholder(idx)"
            >
              <template v-if="stop" #trailing>
                <UButton
                  color="neutral"
                  variant="link"
                  size="sm"
                  icon="i-lucide-circle-x"
                  aria-label="Clear"
                  class="cursor-pointer"
                  @click="clearStop(idx)"
                />
              </template>
            </UInput>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { GeoLocation } from '@/components/types.ts'
import { useVModel } from '@vueuse/core'
import { watchEffect } from 'vue'

const props = defineProps<{
  stops: (GeoLocation | null)[]
}>()

const emit = defineEmits<{
  'update:stops': [value: (GeoLocation | null)[]]
}>()

const stops = useVModel(props, 'stops', emit)

const getStopIconName = (idx: number) =>
  idx === 0 || idx === stops.value.length - 1
    ? 'material-symbols:line-end-circle-outline-rounded'
    : 'lucide:ellipsis'

const getStopPlaceholder = (idx: number) => {
  if (idx === 0) return 'Start'
  else if (idx === stops.value.length - 1) return 'End'
  else return `Stop ${idx + 1}`
}

const getStopIconClass = (idx: number) => {
  if (idx === 0) return '-rotate-90'
  return 'rotate-90'
}

const formatStop = (stop: GeoLocation | null) => {
  if (!stop) return ''

  return `${stop.coords.lat.toFixed(6)}, ${stop.coords.lng.toFixed(6)}`
}

const clearStop = (idx: number) => {
  const nextStops = [...stops.value]
  nextStops[idx] = null
  stops.value = nextStops
}

watchEffect(() => {
  if (stops.value.length >= 2) return
  stops.value = [...stops.value, ...Array.from({ length: Math.max(0, 2 - stops.value.length) }).fill(null)]
})
</script>

<style scoped></style>
