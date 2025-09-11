/*
 Dynamic loader for the generated WASM SDK module.
 We try multiple relative locations to support running from source and from built dist.
*/
// Import types for TS, but load at runtime via require
import type * as WasmModule from '../../pkg/wasm_sdk.js';

// eslint-disable-next-line @typescript-eslint/no-var-requires
const path = require('path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const fs = require('fs');

function resolveWasmModulePath(): string {
  const candidates = [
    path.join(__dirname, 'pkg', 'wasm_sdk.js'), // dist/pkg when bundled
    path.join(__dirname, '../pkg', 'wasm_sdk.js'), // dist sibling pkg
    path.join(__dirname, '../../pkg', 'wasm_sdk.js'), // running from js/src compiled elsewhere
  ];
  for (const p of candidates) {
    try {
      if (fs.existsSync(p)) return p;
    } catch (_) {}
  }
  // Fallback to original relative; may still work in some setups
  return path.join(__dirname, '../../pkg', 'wasm_sdk.js');
}

// eslint-disable-next-line @typescript-eslint/no-var-requires
const mod: typeof WasmModule = require(resolveWasmModulePath());

export = mod;

