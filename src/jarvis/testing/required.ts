/** For tests: unwraps a value the test expects to exist, failing loudly instead of with a type cast. */
export function required<T>(value: T | undefined | null): T {
  if (value === undefined || value === null) throw new Error('expected a value, got nothing');
  return value;
}
