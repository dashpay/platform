import init from './wasm/wasm_dpp';
// Ignore the following line, it's not present in the lib/wasm,
// but added to the dist/wasm by the build process
// @ts-ignore
import wasmBase from './wasm/wasm_dpp_bg';
import * as dpp_module from './dpp';

export * from './dpp';
export type DPPModule = typeof dpp_module;

let isInitialized = false;
let loadingPromise: Promise<void> | null = null;

export default async function loadDpp(): Promise<DPPModule> {
  if (isInitialized) {
    return dpp_module
  }

  if (!loadingPromise) {
    loadingPromise = loadDppModule()
  }

  await loadingPromise;
  isInitialized = true;
  loadingPromise = null;
  return dpp_module;
};

// Decode base64 to Uint8Array using only platform-standard APIs. atob is
// available globally in browsers and in Node ≥18.
const base64ToBytes = (b64: string): Uint8Array => {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
};

const loadDppModule = async () => {
  // @ts-ignore
  const bytes = base64ToBytes(wasmBase);

  if (typeof window !== 'undefined') {
    // Blob's BlobPart typing in lib.dom.d.ts insists on an ArrayBuffer-backed
    // view; our Uint8Array is structurally compatible at runtime.
    const blob = new Blob([bytes as BlobPart], { type: "application/wasm" });
    const wasmUrl = URL.createObjectURL(blob);
    await init(wasmUrl);
  } else {
    dpp_module.initSync({ module: bytes });
  }
}
