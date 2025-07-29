/**
 * Loader for wasm-sdk ES module in CommonJS environment
 * This file must remain as .js to use real dynamic import
 */

async function loadWasmSdk() {
  try {
    // Use actual dynamic import, not transpiled version
    const wasmModule = await import('@dashevo/wasm-sdk');
    return wasmModule;
  } catch (error) {
    throw new Error(`Failed to load wasm-sdk: ${error.message}`);
  }
}

module.exports = { loadWasmSdk };