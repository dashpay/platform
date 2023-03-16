import init from '../wasm/wasm_dpp';
import * as dpp_module from './dpp';
// @ts-ignore
import wasmBase from '../wasm/wasm_dpp_bg.js';
import patchIdentifier from "./identifier/patchIdentifier";

let isInitialized = false;
let loadingPromise: Promise<void> | null = null;

export default async function loadDpp() {
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

const loadDppModule = async () => {
  // @ts-ignore
  let bytes = Buffer.from(wasmBase, 'base64');

  if (typeof fetch !== 'undefined') {
    let blob = new Blob([bytes], { type: "application/wasm" });
    let wasmUrl = URL.createObjectURL(blob);
    await init(wasmUrl);
  } else {
    dpp_module.initSync(bytes);
  }

  patchIdentifier(dpp_module);
}

