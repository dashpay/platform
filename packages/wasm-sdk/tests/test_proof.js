import { readFileSync } from 'fs';
import { WASI } from 'wasi';
import { argv, env } from 'process';

// Create WASI instance
const wasi = new WASI({
  args: argv,
  env,
  preopens: {}
});

// Read and instantiate WASM module
const wasmBuffer = readFileSync('./pkg/wasm_sdk_bg.wasm');
const wasmModule = new WebAssembly.Module(wasmBuffer);
const instance = new WebAssembly.Instance(wasmModule, {
  wasi_snapshot_preview1: wasi.wasiImport
});

// Initialize WASI
wasi.initialize(instance);

console.log('WASM module loaded successfully!');
console.log('Available exports:', Object.keys(instance.exports));