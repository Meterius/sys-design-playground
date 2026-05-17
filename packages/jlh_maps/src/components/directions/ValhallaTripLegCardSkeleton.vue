<template>
  <UCard
    :ui="{
      body: 'p-3 sm:p-3',
    }"
  >
    <div class="grid grid-cols-[2.5rem_auto_minmax(0,1fr)] items-center gap-3">
      <USkeleton class="size-10 rounded-full" />

      <USkeleton class="h-5 w-48" />

      <div class="grid min-w-0 justify-items-end gap-2">
        <USkeleton class="h-4 w-16" />
        <USkeleton class="h-3 w-12" />
      </div>
    </div>
  </UCard>
</template>

<script setup lang="ts"></script>

<style scoped></style>
