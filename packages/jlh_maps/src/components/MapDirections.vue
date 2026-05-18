<template>
  <div class="grid overflow-auto overflow-x-hidden w-full">
    <div class="p-4">
      <h1 class="font-semibold">Directions</h1>
    </div>
    <div class="grid min-h-0">
      <USeparator />
      <div class="grid content-start gap-3 p-4">
        <ModeSelector
          v-model="routeMode"
          :options="routeModeOptions"
          aria-label="Travel mode"
          class="pb-2"
        />

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

        <div
          v-else-if="routeErrorMessage"
          role="alert"
          class="grid grid-cols-[2rem_minmax(0,1fr)] gap-3 rounded-md border border-red-200 bg-red-50 p-3 text-red-800 dark:border-red-900/60 dark:bg-red-950/40 dark:text-red-200"
        >
          <UIcon name="lucide:triangle-alert" class="mt-0.5 size-5" />
          <div class="grid min-w-0 gap-1">
            <h2 class="text-sm font-semibold">Unable to calculate route</h2>
            <p class="text-sm wrap-break-word">{{ routeErrorMessage }}</p>
          </div>
        </div>

        <div v-else-if="route" class="grid gap-3">
          <ValhallaTripLegCard
            v-for="(leg, idx) in route.trip.legs"
            :key="`leg-${idx}`"
            :name="`Route ${idx + 1}`"
            :leg="leg"
            class=""
            @focus-trip="emit('focus-trip', route.trip)"
          />
        </div>

        <USeparator />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import ModeSelector from '@/components/ModeSelector.vue'
import { RouteMode, type GeoLocation } from '@/components/types.ts'
import ValhallaTripLegCard from '@/components/directions/ValhallaTripLegCard.vue'
import ValhallaTripLegCardSkeleton from '@/components/directions/ValhallaTripLegCardSkeleton.vue'
import { useAsyncReactiveRequest } from '@/composables/async-reactive-request.ts'
import { valhallaClient } from '@/external/valhalla.ts'
import { useVModel } from '@vueuse/core'
import { CostingModel, type RouteResponse, type Trip } from 'valhalla_client'
import { computed, ref, watch, watchEffect } from 'vue'

const props = defineProps<{
  stops: (GeoLocation | null)[]
}>()

const emit = defineEmits<{
  'update:stops': [value: (GeoLocation | null)[]]
  'update:trip-primary': [value: Trip | null]
  'update:trip-alternates': [value: Trip[]]
  'focus-trip': [value: Trip]
}>()

const stops = useVModel(props, 'stops', emit)

interface RouteRequestParams {
  stops: GeoLocation[]
  costing: CostingModel
  mode: RouteMode
}

const routeModeDefinitions = [
  { value: RouteMode.Car, label: 'Car', icon: 'lucide:car' },
  { value: RouteMode.Bicycle, label: 'Bicycle', icon: 'lucide:bike' },
  { value: RouteMode.Foot, label: 'Foot', icon: 'lucide:footprints' },
] as const satisfies readonly { value: RouteMode; label: string; icon: string }[]

const routeMode = ref<RouteMode>(RouteMode.Car)

const getRouteCosting = (mode: RouteMode) => {
  switch (mode) {
    case RouteMode.Bicycle:
      return CostingModel.Bicycle
    case RouteMode.Foot:
      return CostingModel.Pedestrian
    case RouteMode.Car:
    default:
      return CostingModel.Auto
  }
}

const hasCompleteStops = (value: (GeoLocation | null)[]): value is GeoLocation[] =>
  value.length >= 2 && value.every((stop): stop is GeoLocation => stop !== null)

const routeStops = computed(() => {
  const value = stops.value
  return hasCompleteStops(value) ? [...value] : null
})

const createRouteRequest = (mode: RouteMode) =>
  useAsyncReactiveRequest<RouteRequestParams | null, RouteResponse | null>(
    computed(() => {
      const value = routeStops.value
      return value ? { stops: value, costing: getRouteCosting(mode), mode } : null
    }),
    async (value, abortSignal) => {
      if (!value) return null

      return valhallaClient
        .route(
          {
            locations: value.stops.map((stop) => ({
              lat: stop.coords.lat,
              lon: stop.coords.lng,
            })),
            costing: value.costing,
          },
          {
            signal: abortSignal,
          },
        )
        .catch((err) => {
          console.error('Valhalla route error', value.mode, err)
          throw err
        })
    },
  )

type RouteRequest = ReturnType<typeof createRouteRequest>

const autoRouteRequest = createRouteRequest(RouteMode.Car)

const routeRequests = {
  [RouteMode.Car]: autoRouteRequest,
  [RouteMode.Bicycle]: createRouteRequest(RouteMode.Bicycle),
  [RouteMode.Foot]: createRouteRequest(RouteMode.Foot),
} satisfies Record<RouteMode, RouteRequest>

const formatRouteDuration = (value: RouteResponse | null) => {
  const seconds = value?.trip.summary.time
  if (typeof seconds !== 'number' || !Number.isFinite(seconds)) return undefined

  const totalMinutes = Math.max(1, Math.round(seconds / 60))
  if (totalMinutes < 60) return `${totalMinutes} min`

  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60

  return minutes === 0 ? `${hours} hr` : `${hours} hr ${minutes} min`
}

const getRouteModeSubLabel = (mode: RouteMode) => {
  const request = routeRequests[mode]
  if (request.loading.value || request.error.value) return undefined

  return formatRouteDuration(request.data.value)
}

const routeModeOptions = computed(() =>
  routeModeDefinitions.map((option) => ({
    ...option,
    subLabel: getRouteModeSubLabel(option.value),
  })),
)

const selectedRouteRequest = computed(() => routeRequests[routeMode.value])
const route = computed(() => selectedRouteRequest.value.data.value)
const loading = computed(() => selectedRouteRequest.value.loading.value)
const routeError = computed(() => selectedRouteRequest.value.error.value)

const routeErrorMessage = computed(() => formatRouteError(routeError.value))

watch(
  route,
  (value) => {
    if (value) {
      console.log('Valhalla route result', value)
    }

    emit('update:trip-primary', value?.trip ?? null)
    emit('update:trip-alternates', value?.alternates?.map((alternate) => alternate.trip) ?? [])
  },
  { immediate: true },
)

watch(
  routeError,
  (value) => {
    if (!value) return

    emit('update:trip-primary', null)
    emit('update:trip-alternates', [])
  },
  { immediate: true },
)

const formatRouteError = (error: unknown) => {
  if (!error) return null
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error

  return 'The routing service returned an unexpected error.'
}

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
