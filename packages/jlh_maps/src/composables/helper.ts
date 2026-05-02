import {watch, type WatchSource} from 'vue'

export function watchDefinedOnce<T>(value: WatchSource<T | undefined>, callback: (value: T) => void) {
  const stop = watch(value, (val) => {
    if (val !== undefined) {
      stop()
      callback(val)
    }
  }, { immediate: true })
}
