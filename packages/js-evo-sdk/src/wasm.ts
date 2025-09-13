// ESM wrapper around @dashevo/wasm-sdk with one-time init
import initWasmSdk, * as wasm from '@dashevo/wasm-sdk';

let initPromise: Promise<void> | undefined;

export async function ensureInitialized(): Promise<void> {
  if (!initPromise) {
    initPromise = initWasmSdk().then(() => undefined);
  }
  return initPromise;
}

// Re-export all wasm SDK symbols for convenience
export * from '@dashevo/wasm-sdk';
export { default } from '@dashevo/wasm-sdk';
