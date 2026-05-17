import { onWatcherCleanup, ref, shallowRef, watch, type WatchSource } from 'vue'

export function useAsyncReactiveRequest<K, V>(
  params: WatchSource<K>,
  fetch: (params: K, abortSignal: AbortSignal) => Promise<V>,
) {
  const loading = ref(false)

  const error = shallowRef<unknown | null>(null)
  const data = shallowRef<V | null>(null)

  watch(
    params,
    async (value) => {
      const abortController = new AbortController()

      onWatcherCleanup(() => {
        abortController.abort()
      })

      try {
        loading.value = true
        data.value = await fetch(value, abortController.signal)
        error.value = null
      } catch (caughtError: unknown) {
        if (!abortController.signal.aborted) {
          error.value = caughtError
        }
      } finally {
        loading.value = false
      }
    },
    { immediate: true },
  )

  return {
    loading,
    data,
    error,
  }
}
