let wasmSdkModule: any = null;
let initPromise: Promise<void> | null = null;

export async function loadWasmSdk(): Promise<any> {
  if (wasmSdkModule) {
    console.log('WASM SDK already loaded, returning cached instance');
    return wasmSdkModule;
  }

  if (initPromise) {
    console.log('WASM SDK initialization already in progress, waiting...');
    await initPromise;
    return wasmSdkModule;
  }

  console.log('Starting WASM SDK initialization...');
  
  initPromise = (async () => {
    try {
      // Dynamic import to enable code splitting
      console.log('Attempting to import WASM module from ../../wasm/wasm_sdk');
      const wasm = await import('../../wasm/wasm_sdk');
      
      console.log('WASM module imported, initializing...');
      console.log('Available exports:', Object.keys(wasm));
      
      await wasm.default();
      
      console.log('WASM SDK initialized successfully');
      wasmSdkModule = wasm;
    } catch (error) {
      console.error('Failed to load WASM SDK:', error);
      console.error('Error details:', {
        message: error.message,
        stack: error.stack,
        name: error.name
      });
      
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