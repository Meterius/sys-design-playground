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

        <USeparator />

        <div v-if="loading" class="grid gap-3">
          <ValhallaTripLegCardSkeleton />
        </div>

        <div v-else-if="route" class="grid gap-3">
          <ValhallaTripLegCard
            v-for="(leg, idx) in route.trip.legs"
            :key="`leg-${idx}`"
            :name="`Route ${idx + 1}`"
            :leg="leg"
            class=""
          />
        </div>

        <USeparator />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { GeoLocation } from '@/components/types.ts'
import ValhallaTripLegCard from '@/components/directions/ValhallaTripLegCard.vue'
import ValhallaTripLegCardSkeleton from '@/components/directions/ValhallaTripLegCardSkeleton.vue'
import { useAsyncReactiveRequest } from '@/composables/async-reactive-request.ts'
import { valhallaClient } from '@/external/valhalla.ts'
import { useVModel } from '@vueuse/core'
import { CostingModel, type RouteResponse } from 'valhalla_client'
import { computed, watch, watchEffect } from 'vue'

const props = defineProps<{
  stops: (GeoLocation | null)[]
}>()

const emit = defineEmits<{
  'update:stops': [value: (GeoLocation | null)[]]
}>()

const stops = useVModel(props, 'stops', emit)

const hasCompleteStops = (value: (GeoLocation | null)[]): value is GeoLocation[] =>
  value.length >= 2 && value.every((stop): stop is GeoLocation => stop !== null)

const routeStops = computed(() => {
  const value = stops.value
  return hasCompleteStops(value) ? [...value] : null
})

const { data: route, loading } = useAsyncReactiveRequest<
  GeoLocation[] | null,
  RouteResponse | null
>(routeStops, async (value, abortSignal) => {
  if (!value) return null

  return valhallaClient
    .route(
      {
        locations: value.map((stop) => ({
          lat: stop.coords.lat,
          lon: stop.coords.lng,
        })),
        costing: CostingModel.Auto,
      },
      {
        signal: abortSignal,
      },
    )
    .catch((err) => {
      console.error('Valhalla route error', err)
      throw err
    })
})

watch(route, (value) => {
  if (value) {
    console.log('Valhalla route result', value)
  }
})

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
  stops.value = [
    ...stops.value,
    ...Array.from<null>({ length: Math.max(0, 2 - stops.value.length) }).fill(null),
  ]
})
</script>

<style scoped></style>
