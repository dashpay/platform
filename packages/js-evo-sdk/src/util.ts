export function asJsonString(value: unknown): string | undefined {
  if (value == null) return undefined;
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
}
