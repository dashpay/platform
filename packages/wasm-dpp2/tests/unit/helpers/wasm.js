import init, * as wasmModule from '../../../dist/dpp.compressed.js';

let initialized = false;
let initPromise;

/**
 *
 */
async function ensureInitialized() {
  if (initialized) {
    return;
  }

  if (!initPromise) {
    initPromise = init();
  }

  await initPromise;
  initialized = true;
}

/**
 *
 */
export async function getWasm() {
  await ensureInitialized();
  return wasmModule;
}

export default getWasm;
