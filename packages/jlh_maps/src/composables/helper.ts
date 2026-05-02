import { watchEffect } from 'vue'

export function watchOnceDefined<T>(getValue: () => T | undefined, callback: (value: T) => void) {
  const stop = watchEffect(() => {
    const value = getValue()

    if (value !== undefined) {
      stop()
      callback(value)
    }
  })
}
