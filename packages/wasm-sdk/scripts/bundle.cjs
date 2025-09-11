#!/usr/bin/env node
/*
  Prepare publishable artifacts with a single-file SDK wrapper and raw outputs.
  - Input: pkg/wasm_sdk.js, pkg/wasm_sdk_bg.wasm, pkg/wasm_sdk.d.ts
  - Output: dist/raw/* (unaltered wasm-bindgen outputs), dist/sdk.js (single-file wrapper with inlined WASM), dist/sdk.d.ts

  Notes:
  - We keep the exported API identical to wasm_bindgen output, including default export (init) and initSync.
  - Wrapper inlines WASM as base64 and provides `await init()` for both Node and browser.
  - Node: uses inlined bytes with initSync under the hood (still awaitable).
  - Browser: compiles in a Web Worker (fallback to async compile on main thread) and then instantiates.
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

// Ensure dist directories and write outputs
fs.mkdirSync(distDir, { recursive: true });
fs.mkdirSync(rawDir, { recursive: true });
// Copy raw wasm-bindgen outputs
const rawJs = fs.readFileSync(jsPath, 'utf8');
fs.writeFileSync(path.join(rawDir, 'wasm_sdk.js'), rawJs);
fs.copyFileSync(wasmPath, path.join(rawDir, 'wasm_sdk_bg.wasm'));
fs.copyFileSync(dtsPath, path.join(rawDir, 'wasm_sdk.d.ts'));
if (fs.existsSync(wasmDtsPath)) {
  fs.copyFileSync(wasmDtsPath, path.join(rawDir, 'wasm_sdk_bg.wasm.d.ts'));
}

// Produce a sanitized variant of wasm_bindgen JS without the default new URL('...wasm') path,
// so downstream bundlers importing the wrapper won't see a literal asset URL and won't emit the .wasm file.
const defaultUrlRegex = /if\s*\(\s*typeof\s+module_or_path\s*===\s*'undefined'\s*\)\s*\{\s*module_or_path\s*=\s*new\s+URL\('wasm_sdk_bg\\.wasm',\s*import\\.meta\\.url\);\s*\}/;
const sanitizedJs = rawJs.replace(defaultUrlRegex, "if (typeof module_or_path === 'undefined') { }");
fs.writeFileSync(path.join(rawDir, 'wasm_sdk.no_url.js'), sanitizedJs);

// Build single-file wrapper with inlined WASM and worker-based compile in the browser
const wasmBase64 = fs.readFileSync(wasmPath).toString('base64');
const wrapper = `// Single-file ESM wrapper around wasm-bindgen output.\n// - Inlines WASM bytes as base64.\n// - Exposes async default init() for both Node and browser.\n// - Browser compiles in a Web Worker and instantiates on main thread (fallback to async compile).\n// - Node uses initSync with inlined bytes (still awaitable for uniform API).\n\nimport rawInit, { initSync as rawInitSync } from './raw/wasm_sdk.no_url.js';\n\nexport * from './raw/wasm_sdk.no_url.js';\nexport { initSync } from './raw/wasm_sdk.no_url.js';\n\nconst __WASM_BASE64 = '${wasmBase64}';\nfunction __wasmBytes() {\n  if (typeof Buffer !== 'undefined' && typeof Buffer.from === 'function') {\n    return Buffer.from(__WASM_BASE64, 'base64');\n  }\n  const atobFn = (typeof atob === 'function') ? atob : (s) => globalThis.atob(s);\n  const bin = atobFn(__WASM_BASE64);\n  const len = bin.length;\n  const bytes = new Uint8Array(len);\n  for (let i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);\n  return bytes;\n}\n\nfunction __supportsWorker() {\n  return typeof Worker !== 'undefined' && typeof Blob !== 'undefined' && typeof URL !== 'undefined';\n}\n\nasync function __compileInWorker(bytes) {\n  if (!__supportsWorker()) {\n    return WebAssembly.compile(bytes);\n  }\n  const src = 'self.onmessage=async(e)=>{try{const m=await WebAssembly.compile(e.data);self.postMessage({ok:1,mod:m});}catch(err){self.postMessage({ok:0,err:String(err)});}}';\n  const blob = new Blob([src], { type: 'application/javascript' });\n  const url = URL.createObjectURL(blob);\n  return new Promise((resolve) => {\n    const w = new Worker(url);\n    w.onmessage = (ev) => {\n      URL.revokeObjectURL(url);\n      w.terminate();\n      const d = ev.data || {};\n      if (d.ok && d.mod) {\n        resolve(d.mod);\n      } else {\n        resolve(WebAssembly.compile(bytes));\n      }\n    };\n    // Transfer the underlying buffer to avoid copy\n    try {\n      w.postMessage(bytes.buffer, [bytes.buffer]);\n    } catch (_) {\n      // If transfer fails (detached), send a copy\n      w.postMessage(new Uint8Array(bytes));\n    }\n  });\n}\n\nconst isNode = typeof window === 'undefined' && typeof process !== 'undefined' && !!(process.versions && process.versions.node);\n\nexport default async function init(moduleOrPath) {\n  if (isNode) {\n    if (typeof moduleOrPath === 'undefined') {\n      const bytes = __wasmBytes();\n      return rawInitSync({ module: bytes });\n    }\n    return rawInit(moduleOrPath);\n  }\n  if (typeof moduleOrPath === 'undefined') {\n    const bytes = __wasmBytes();\n    let mod;\n    try {\n      mod = await __compileInWorker(bytes);\n    } catch (_) {\n      mod = await WebAssembly.compile(bytes);\n    }\n    return rawInit({ module_or_path: mod });\n  }\n  return rawInit(moduleOrPath);\n}\n`;
fs.writeFileSync(path.join(distDir, 'sdk.js'), wrapper);
fs.copyFileSync(dtsPath, path.join(distDir, 'sdk.d.ts'));

// Basic report
const outStat = fs.statSync(path.join(distDir, 'sdk.js'));
console.log(`Wrote dist/sdk.js (${outStat.size} bytes) single-file wrapper (inline WASM)`);
console.log('Wrote dist/sdk.d.ts');
console.log('Wrote dist/raw/* (separate JS + WASM)');

// Clean up: remove pkg directory after bundling to avoid publishing it
try {
  fs.rmSync(pkgDir, { recursive: true, force: true });
  console.log('Removed pkg/ directory after bundling');
} catch (e) {
  console.warn('Warning: failed to remove pkg/ directory:', e?.message || e);
}
