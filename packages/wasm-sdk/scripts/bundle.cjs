#!/usr/bin/env node
/*
  Bundle the wasm-bindgen JS glue and WASM binary into a single ESM file.
  - Input: pkg/wasm_sdk.js, pkg/wasm_sdk_bg.wasm, pkg/wasm_sdk.d.ts
  - Output: dist/sdk.js (single-file with embedded WASM), dist/sdk.d.ts

  Notes:
  - We keep the exported API identical to wasm_bindgen output, including default export (init) and initSync.
  - We replace the default loader path with inlined bytes so no network or file access is required at runtime.
*/

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const pkgDir = path.join(root, 'pkg');
const distDir = path.join(root, 'dist');
const rawDir = path.join(distDir, 'raw');

const jsPath = path.join(pkgDir, 'wasm_sdk.js');
const wasmPath = path.join(pkgDir, 'wasm_sdk_bg.wasm');
const dtsPath = path.join(pkgDir, 'wasm_sdk.d.ts');
const wasmDtsPath = path.join(pkgDir, 'wasm_sdk_bg.wasm.d.ts');

if (!fs.existsSync(jsPath) || !fs.existsSync(wasmPath) || !fs.existsSync(dtsPath)) {
  console.error('Missing build artifacts in pkg/. Run build first.');
  process.exit(1);
}

const js = fs.readFileSync(jsPath, 'utf8');
const wasmBase64 = fs.readFileSync(wasmPath).toString('base64');

// Helper injected to decode base64 → Uint8Array in both Node and browser
const injectHeader = `
// Inlined WASM bytes (base64)
const __WASM_BASE64 = '${wasmBase64}';
function __wasmBytes() {
  if (typeof Buffer !== 'undefined' && typeof Buffer.from === 'function') {
    return Buffer.from(__WASM_BASE64, 'base64');
  }
  const atobFn = (typeof atob === 'function') ? atob : (s) => globalThis.atob(s);
  const bin = atobFn(__WASM_BASE64);
  const len = bin.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}
`;

// Patch 1: default init path resolution to use inlined bytes
const initDefaultSearch = /if \(typeof module_or_path === 'undefined'\) {\s*\n\s*module_or_path = new URL\('wasm_sdk_bg\.wasm', import\.meta\.url\);\s*\n\s*}/;
const initDefaultReplace = `if (typeof module_or_path === 'undefined') {\n        module_or_path = __wasmBytes();\n    }`;

// Patch 2: initSync to use inlined bytes when module is not provided
const initSyncSearch = /(__wbg_init_memory\(imports\);\s*\n\s*if \(!\(module instanceof WebAssembly\.Module\)\) {)/;
const initSyncReplace = `__wbg_init_memory(imports);\n\n    if (typeof module === 'undefined') {\n        module = __wasmBytes();\n    }\n\n    if (!(module instanceof WebAssembly.Module)) {`;

let patched = js;

if (!initDefaultSearch.test(patched)) {
  console.error('Failed to find default init block to patch');
  process.exit(1);
}
patched = patched.replace(initDefaultSearch, initDefaultReplace);

if (!initSyncSearch.test(patched)) {
  console.error('Failed to find initSync block to patch');
  process.exit(1);
}
patched = patched.replace(initSyncSearch, initSyncReplace);

// Prepend helper header
patched = injectHeader + '\n' + patched;

// Ensure dist directories and write outputs
fs.mkdirSync(distDir, { recursive: true });
fs.mkdirSync(rawDir, { recursive: true });
fs.writeFileSync(path.join(distDir, 'sdk.js'), patched);
fs.copyFileSync(dtsPath, path.join(distDir, 'sdk.d.ts'));

// Also ship non-bundled artifacts for advanced/asset-pipeline users
fs.copyFileSync(jsPath, path.join(rawDir, 'wasm_sdk.js'));
fs.copyFileSync(wasmPath, path.join(rawDir, 'wasm_sdk_bg.wasm'));
fs.copyFileSync(dtsPath, path.join(rawDir, 'wasm_sdk.d.ts'));
if (fs.existsSync(wasmDtsPath)) {
  fs.copyFileSync(wasmDtsPath, path.join(rawDir, 'wasm_sdk_bg.wasm.d.ts'));
}

// Basic report
const outStat = fs.statSync(path.join(distDir, 'sdk.js'));
console.log(`Wrote dist/sdk.js (${outStat.size} bytes) with inlined WASM (${Math.round(Buffer.byteLength(wasmBase64, 'utf8')/1024)} KB base64)`);
console.log('Wrote dist/sdk.d.ts');
console.log('Wrote dist/raw/* (separate JS + WASM)');

// Clean up: remove pkg directory after bundling to avoid publishing it
try {
  fs.rmSync(pkgDir, { recursive: true, force: true });
  console.log('Removed pkg/ directory after bundling');
} catch (e) {
  console.warn('Warning: failed to remove pkg/ directory:', e?.message || e);
}
