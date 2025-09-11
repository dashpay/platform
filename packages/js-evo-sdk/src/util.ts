// Shared helpers and types for the TypeScript facades
import * as wasm from './wasm';

export type Wasm = typeof wasm;

export function asJsonString(value: unknown): string | undefined {
  if (value == null) return undefined;
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
}
