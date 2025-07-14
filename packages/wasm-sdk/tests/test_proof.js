import { readFileSync, existsSync } from 'fs';
import { WASI } from 'wasi';
import { argv, env } from 'process';
import { join } from 'path';

// Create WASI instance
const wasi = new WASI({
  args: argv,
  env,
  preopens: {}
});

// Read and instantiate WASM module
let wasmBuffer, wasmModule, instance;

// Check if WASM file exists before attempting to read it
const wasmPath = './pkg/wasm_sdk_bg.wasm';
if (!existsSync(wasmPath)) {
  console.error('Error: WASM file not found at:', wasmPath);
  console.error('Please build the WASM module first by running: ./build.sh');
  process.exit(1);
}

try {
  wasmBuffer = readFileSync(wasmPath);
  console.log('WASM file read successfully');
} catch (error) {
  console.error('Failed to read WASM file:', error.message);
  console.error('Make sure you have built the WASM module with: ./build.sh');
  process.exit(1);
}

try {
  wasmModule = new WebAssembly.Module(wasmBuffer);
  console.log('WASM module created successfully');
} catch (error) {
  console.error('Failed to create WebAssembly module:', error.message);
  console.error('The WASM file may be corrupted. Try rebuilding with: ./build.sh');
  process.exit(1);
}

try {
  instance = new WebAssembly.Instance(wasmModule, {
    wasi_snapshot_preview1: wasi.wasiImport
  });
  console.log('WASM instance created successfully');
} catch (error) {
  console.error('Failed to instantiate WebAssembly module:', error.message);
  console.error('There may be import/export mismatches or runtime issues');
  process.exit(1);
}

// Initialize WASI
wasi.initialize(instance);

console.log('WASM module loaded successfully!');
console.log('Available exports:', Object.keys(instance.exports));