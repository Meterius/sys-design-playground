<template>
  <div class="relative isolate" role="radiogroup" :aria-label="props.ariaLabel">
    <UCard
      :ui="{
        body: 'flex !p-0 sm:!p-0',
      }"
      class="relative z-10"
    >
      <UButton
        v-for="option in props.options"
        :key="'button-' + option.value"
        type="button"
        :color="model === option.value ? 'primary' : 'neutral'"
        :variant="model === option.value ? 'solid' : 'ghost'"
        size="sm"
        block
        role="radio"
        :class="['rounded-none py-3', model === option.value ? '' : 'cursor-pointer text-muted']"
        :aria-checked="model === option.value"
        :title="option.label"
        @click="model = option.value"
      >
        <span class="inline-flex min-w-0 items-center gap-1.5">
          <UIcon v-if="option.icon" :name="option.icon" class="size-4 shrink-0" />
          <span class="truncate">{{ option.label }}</span>
        </span>
      </UButton>
    </UCard>

    <div class="relative z-0 -mt-px flex">
      <div
        v-for="option in props.options"
        :key="'badge-' + option.value"
        class="flex min-w-0 flex-1 justify-center"
      >
        <Transition
          enter-active-class="transition duration-150 ease-out"
          enter-from-class="-translate-y-1 opacity-0"
          enter-to-class="translate-y-0 opacity-100"
          leave-active-class="transition duration-100 ease-in"
          leave-from-class="translate-y-0 opacity-100"
          leave-to-class="-translate-y-1 opacity-0"
        >
          <UBadge
            v-if="option.subLabel"
            :label="option.subLabel"
            color="neutral"
            variant="soft"
            size="sm"
            class="pointer-events-none font-normal max-w-[calc(100%-0.5rem)] self-center rounded-t-none"
          />
        </Transition>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts" generic="T extends string">
type ModeSelectorOption<T extends string = string> = {
  value: T
  label: string
  icon?: string
  subLabel?: string
}

const props = withDefaults(
  defineProps<{
    options: readonly ModeSelectorOption<T>[]
    ariaLabel?: string
  }>(),
  {
    ariaLabel: 'Mode',
  },
)

const model = defineModel<T>({ required: true })
</script>
