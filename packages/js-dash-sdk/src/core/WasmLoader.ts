let wasmSdkModule: any = null;
let initPromise: Promise<void> | null = null;

export async function loadWasmSdk(): Promise<any> {
  if (wasmSdkModule) {
    return wasmSdkModule;
  }

  if (initPromise) {
    await initPromise;
    return wasmSdkModule;
  }

  initPromise = (async () => {
    try {
      // Dynamic import to enable code splitting
      const wasm = await import('../../wasm/wasm_sdk');
      await wasm.default();
      wasmSdkModule = wasm;
    } catch (error) {
      console.error('Failed to load WASM SDK:', error);
      // Reset promise on failure to allow retry
      initPromise = null;
      throw new Error('Failed to initialize WASM SDK. Please ensure WASM is supported in your environment.');
    }
  })();

  await initPromise;
  return wasmSdkModule;
}

export function getWasmSdk(): any {
  if (!wasmSdkModule) {
    throw new Error('WASM SDK not initialized. Call loadWasmSdk() first.');
  }
  return wasmSdkModule;
}

export function isWasmLoaded(): boolean {
  return wasmSdkModule !== null;
}