import { watch, type WatchSource } from 'vue'

export function watchDefinedOnce<T>(
  value: WatchSource<T | undefined>,
  callback: (value: T) => void,
) {
  watch(
    value,
    (val, prev) => {
      if (val !== undefined && prev === undefined) {
        callback(val)
      }
    },
    { immediate: true },
  )
}
