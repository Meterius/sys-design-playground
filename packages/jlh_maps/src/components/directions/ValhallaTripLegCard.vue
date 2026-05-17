<template>
  <UCard
    :ui="{
      body: 'p-3 sm:p-3',
    }"
  >
    <div class="grid gap-2">
      <div class="grid grid-cols-[2.5rem_auto_minmax(0,1fr)] items-center gap-3">
        <div class="grid size-10 place-items-center">
          <UIcon :name="travelModeIcon" class="size-6" />
        </div>

        <div>
          <h3>{{ name }}</h3>
        </div>

        <div class="grid min-w-0 justify-items-end gap-0.5 text-right">
          <div class="text-sm font-semibold text-highlighted">{{ formattedTime }}</div>
          <div class="text-xs text-muted">{{ formattedDistance }}</div>
        </div>
      </div>

      <div class="grid gap-2 pl-[3.25rem]">
        <UCollapsible>
          <UButton
            class="w-full justify-between px-0 cursor-pointer"
            color="neutral"
            variant="link"
            label="Instructions"
            trailing-icon="lucide:chevron-down"
          />

          <template #content>
            <p class="pb-2 text-xs text-muted">
              Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer posuere erat a ante.
            </p>
          </template>
        </UCollapsible>

        <UCollapsible>
          <UButton
            class="w-full justify-between px-0 cursor-pointer"
            color="neutral"
            variant="link"
            label="Info"
            trailing-icon="lucide:chevron-down"
          />

          <template #content>
            <p class="pb-2 text-xs text-muted">
              Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer posuere erat a ante.
            </p>
          </template>
        </UCollapsible>
      </div>
    </div>
  </UCard>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { TravelMode, type TripLeg } from 'valhalla_client'

const props = defineProps<{
  name: string
  leg: TripLeg
}>()

const majorityTravelMode = computed(() => {
  const totals = new Map<string, number>()

  for (const maneuver of props.leg.maneuvers ?? []) {
    const mode = maneuver.travel_mode ?? TravelMode.Drive
    totals.set(mode, (totals.get(mode) ?? 0) + maneuver.time)
  }

  return (
    [...totals.entries()].sort(([, left], [, right]) => right - left)[0]?.[0] ?? TravelMode.Drive
  )
})

const travelModeIcon = computed(() => {
  switch (majorityTravelMode.value) {
    case TravelMode.Pedestrian:
      return 'lucide:footprints'
    case TravelMode.Bicycle:
    case TravelMode.Bikeshare:
      return 'lucide:bike'
    case TravelMode.Transit:
      return 'lucide:bus'
    case TravelMode.Drive:
    default:
      return 'lucide:car'
  }
})

const formattedTime = computed(() => {
  const totalMinutes = Math.round(props.leg.summary.time / 60)

  if (totalMinutes < 60) return `${totalMinutes} min`

  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  return minutes === 0 ? `${hours} hr` : `${hours} hr ${minutes} min`
})

const formattedDistance = computed(() => {
  const distance = props.leg.summary.length
  return distance < 10 ? `${distance.toFixed(1)} km` : `${Math.round(distance)} km`
})
</script>

<style scoped></style>
