export function guardArray<S, T extends S>(
  guard: (value: S) => value is T,
  array: S[],
): array is T[] {
  return array.every(guard)
}

export function isNonNullable<T>(value: T | null | undefined): value is T {
  return value !== null && value !== undefined
}
