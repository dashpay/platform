#!/usr/bin/env node
/*
  Prepare publishable artifacts for wasm-dpp2 with single-file wrappers and raw outputs.
  - Input: pkg/wasm_dpp2.js, pkg/wasm_dpp2_bg.wasm, pkg/wasm_dpp2.d.ts
  - Output: dist/raw/* (unaltered wasm-bindgen outputs), dist/dpp.js (single-file wrapper with inlined WASM), dist/dpp.compressed.js, dist/dpp.d.ts
*/

const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const root = process.cwd();
const pkgDir = path.join(root, 'pkg');
const distDir = path.join(root, 'dist');
const rawDir = path.join(distDir, 'raw');

const jsPath = path.join(pkgDir, 'wasm_dpp2.js');
const wasmPath = path.join(pkgDir, 'wasm_dpp2_bg.wasm');
const dtsPath = path.join(pkgDir, 'wasm_dpp2.d.ts');
const wasmDtsPath = path.join(pkgDir, 'wasm_dpp2_bg.wasm.d.ts');

if (!fs.existsSync(jsPath) || !fs.existsSync(wasmPath) || !fs.existsSync(dtsPath)) {
  console.error('Missing build artifacts in pkg/. Run build first.');
  process.exit(1);
}

fs.mkdirSync(distDir, { recursive: true });
fs.mkdirSync(rawDir, { recursive: true });

const rawJs = fs.readFileSync(jsPath, 'utf8');
fs.writeFileSync(path.join(rawDir, 'wasm_dpp2.js'), rawJs);
fs.copyFileSync(wasmPath, path.join(rawDir, 'wasm_dpp2_bg.wasm'));
fs.copyFileSync(dtsPath, path.join(rawDir, 'wasm_dpp2.d.ts'));
if (fs.existsSync(wasmDtsPath)) {
  fs.copyFileSync(wasmDtsPath, path.join(rawDir, 'wasm_dpp2_bg.wasm.d.ts'));
}

const defaultUrlRegex = /if\s*\(\s*typeof\s+module_or_path\s*===\s*'undefined'\s*\)\s*\{\s*module_or_path\s*=\s*new\s+URL\('wasm_dpp2_bg\\.wasm',\s*import\\.meta\\.url\);\s*\}/;
const sanitizedJs = rawJs.replace(defaultUrlRegex, "if (typeof module_or_path === 'undefined') { }");
fs.writeFileSync(path.join(rawDir, 'wasm_dpp2.no_url.js'), sanitizedJs);

const wasmBytes = fs.readFileSync(wasmPath);
const wasmBase64 = wasmBytes.toString('base64');
const wasmGzip = zlib.gzipSync(wasmBytes, { level: zlib.constants.Z_BEST_COMPRESSION });
const wasmGzipBase64 = wasmGzip.toString('base64');

const wrapper = `// Single-file ESM wrapper around wasm-bindgen output.\n// - Inlines WASM bytes as base64.\n// - Exposes async default init() for both Node and browser.\n\nimport rawInit, { initSync as rawInitSync } from './raw/wasm_dpp2.no_url.js';\n\nexport * from './raw/wasm_dpp2.no_url.js';\nexport { initSync } from './raw/wasm_dpp2.no_url.js';\n\nconst __WASM_BASE64 = '${wasmBase64}';\nfunction __wasmBytes() {\n  if (typeof Buffer !== 'undefined' && typeof Buffer.from === 'function') {\n    return Buffer.from(__WASM_BASE64, 'base64');\n  }\n  const atobFn = (typeof atob === 'function') ? atob : (s) => globalThis.atob(s);\n  const bin = atobFn(__WASM_BASE64);\n  const len = bin.length;\n  const bytes = new Uint8Array(len);\n  for (let i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);\n  return bytes;\n}\n\nfunction __supportsWorker() {\n  return typeof Worker !== 'undefined' && typeof Blob !== 'undefined' && typeof URL !== 'undefined';\n}\n\nasync function __compileInWorker(bytes) {\n  if (!__supportsWorker()) {\n    return WebAssembly.compile(bytes);\n  }\n  const src = 'self.onmessage=async(e)=>{try{const m=await WebAssembly.compile(e.data);self.postMessage({ok:1,mod:m});}catch(err){self.postMessage({ok:0,err:String(err)});}}';\n  const blob = new Blob([src], { type: 'application/javascript' });\n  const url = URL.createObjectURL(blob);\n  return new Promise((resolve, reject) => {\n    const w = new Worker(url);\n    const cleanup = () => {\n      URL.revokeObjectURL(url);\n      w.terminate();\n    };\n    w.onmessage = (ev) => {\n      const d = ev.data || {};\n      if (d.ok && d.mod) {\n        cleanup();\n        resolve(d.mod);\n      } else {\n        cleanup();\n        reject(new Error(d.err || 'Worker failed to compile WASM.'));
      }\n    };\n    w.onerror = (err) => {\n      cleanup();\n      reject(err instanceof Error ? err : new Error(String(err && err.message ? err.message : err)));\n    };\n    try {\n      w.postMessage(bytes.buffer, [bytes.buffer]);\n    } catch (_) {\n      w.postMessage(new Uint8Array(bytes));\n    }\n  });\n}\n\nconst isNode = typeof window === 'undefined' && typeof process !== 'undefined' && !!(process.versions && process.versions.node);\n\nexport default async function init(moduleOrPath) {\n  if (isNode) {\n    if (typeof moduleOrPath === 'undefined') {\n      const bytes = __wasmBytes();\n      return rawInitSync({ module: bytes });\n    }\n    return rawInit(moduleOrPath);\n  }\n  if (typeof moduleOrPath === 'undefined') {\n    const bytes = __wasmBytes();\n    let mod;\n    try {\n      mod = await __compileInWorker(bytes);\n    } catch (_) {\n      mod = await WebAssembly.compile(bytes);\n    }\n    return rawInit({ module_or_path: mod });\n  }\n  return rawInit(moduleOrPath);\n}\n`;
fs.writeFileSync(path.join(distDir, 'dpp.js'), wrapper);

const compressedWrapper = `// Gzip-compressed single-file ESM wrapper around wasm-bindgen output.\n// - Inlines WASM as base64 gzip payload to reduce bundle size.\n\nimport rawInit, { initSync as rawInitSync } from './raw/wasm_dpp2.no_url.js';\n\nexport * from './raw/wasm_dpp2.no_url.js';\nexport { initSync } from './raw/wasm_dpp2.no_url.js';\n\nconst __WASM_COMPRESSED_BASE64 = '${wasmGzipBase64}';\nconst __WASM_COMPRESSION = 'gzip';\nconst isNode = typeof window === 'undefined' && typeof process !== 'undefined' && !!(process.versions && process.versions.node);\n\nfunction __decodeBase64(source) {\n  if (typeof Buffer !== 'undefined' && typeof Buffer.from === 'function') {\n    return Buffer.from(source, 'base64');\n  }\n  const atobFn = (typeof atob === 'function') ? atob : (s) => globalThis.atob(s);\n  const bin = atobFn(source);\n  const len = bin.length;\n  const bytes = new Uint8Array(len);\n  for (let i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);\n  return bytes;\n}\n\nlet __nodeZlibPromise;\nfunction __loadNodeZlib() {\n  if (!__nodeZlibPromise) {\n    const importer = new Function('return import(\\"node:zlib\\")');\n    __nodeZlibPromise = importer();\n  }\n  return __nodeZlibPromise;\n}\n\nasync function __decompress(bytes) {\n  if (!__WASM_COMPRESSION) {\n    return bytes;\n  }\n  if (isNode) {\n    const { gunzipSync } = await __loadNodeZlib();\n    const out = gunzipSync(bytes);
    return out instanceof Uint8Array ? out : new Uint8Array(out.buffer, out.byteOffset, out.byteLength);\n  }\n  if (typeof Blob === 'function' && typeof Response === 'function' && typeof DecompressionStream === 'function') {\n    const res = new Response(\n      new Blob([bytes]).stream().pipeThrough(new DecompressionStream(__WASM_COMPRESSION))\n    );\n    const buf = await res.arrayBuffer();\n    return new Uint8Array(buf);\n  }\n  throw new Error('Gzip decompression not supported in this environment.');\n}\n\nasync function __wasmBytes(options = {}) {\n  const { decompress = true } = options;\n  if (!__WASM_COMPRESSION) {\n    throw new Error('Compression metadata missing.');\n  }\n  const compressed = __decodeBase64(__WASM_COMPRESSED_BASE64);\n  if (!decompress) {\n    return compressed;\n  }\n  return __decompress(compressed);\n}\n\nfunction __supportsWorker() {\n  return typeof Worker !== 'undefined' && typeof Blob !== 'undefined' && typeof URL !== 'undefined';\n}\n\nasync function __compileInWorker(compressedBytes) {\n  const bytes = compressedBytes instanceof Uint8Array ? compressedBytes : new Uint8Array(compressedBytes);\n  if (!__supportsWorker()) {\n    const decompressed = await __decompress(bytes);\n    return WebAssembly.compile(decompressed);\n  }\n  const src = "self.onmessage=async(event)=>{try{const data=event.data||{};let bytes=data.compressed;const compression=data.compression||null;if(!(bytes instanceof Uint8Array)){bytes=bytes?new Uint8Array(bytes):new Uint8Array();}if(compression){if(typeof Blob==='function'&&typeof Response==='function'&&typeof DecompressionStream==='function'){const res=new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream(compression)));const buf=await res.arrayBuffer();bytes=new Uint8Array(buf);}else{throw new Error('DecompressionStream not available');}}const mod=await WebAssembly.compile(bytes);self.postMessage({ok:1,mod});}catch(err){self.postMessage({ok:0,err:String(err)})}}";
  const blob = new Blob([src], { type: 'application/javascript' });
  const url = URL.createObjectURL(blob);
  return new Promise((resolve, reject) => {
    const worker = new Worker(url);
    const cleanup = () => {
      URL.revokeObjectURL(url);
      worker.terminate();
    };
    worker.onmessage = (ev) => {
      const d = ev.data || {};
      if (d.ok && d.mod) {
        cleanup();
        resolve(d.mod);
      } else {
        cleanup();
        reject(new Error(d.err || 'Worker failed to compile WASM.'));
      }
    };
    worker.onerror = (err) => {
      cleanup();
      reject(err instanceof Error ? err : new Error(String(err && err.message ? err.message : err)));
    };
    try {
      worker.postMessage({ compressed: bytes, compression: __WASM_COMPRESSION });
    } catch (postErr) {
      cleanup();
      reject(postErr);
    }
  });
}

export default async function init(moduleOrPath) {
  if (isNode) {
    if (typeof moduleOrPath === 'undefined') {
      const bytes = await __wasmBytes();
      return rawInitSync({ module: bytes });
    }
    return rawInit(moduleOrPath);
  }
  if (typeof moduleOrPath === 'undefined') {
    const compressedBytes = await __wasmBytes({ decompress: false });
    let mod;
    try {
      mod = await __compileInWorker(compressedBytes);
    } catch (_) {
      const decompressed = await __decompress(compressedBytes);
      mod = await WebAssembly.compile(decompressed);
    }
    return rawInit({ module_or_path: mod });
  }
  return rawInit(moduleOrPath);
}
`;
fs.writeFileSync(path.join(distDir, 'dpp.compressed.js'), compressedWrapper);

const dppJsPath = path.join(distDir, 'dpp.js');
const dppCompressedPath = path.join(distDir, 'dpp.compressed.js');

const baseStat = fs.statSync(dppJsPath);
const compressedStat = fs.statSync(dppCompressedPath);
const baseGzipSize = zlib.gzipSync(fs.readFileSync(dppJsPath)).length;
const compressedGzipSize = zlib.gzipSync(fs.readFileSync(dppCompressedPath)).length;

fs.copyFileSync(dtsPath, path.join(distDir, 'dpp.d.ts'));

console.log(`Wrote dist/dpp.js (${baseStat.size} bytes) single-file wrapper (inline WASM)`);
console.log(`Wrote dist/dpp.compressed.js (${compressedStat.size} bytes) gzip inline wrapper`);
console.log(`gzip(dpp.js): ${baseGzipSize} bytes | gzip(dpp.compressed.js): ${compressedGzipSize} bytes`);
console.log('Wrote dist/dpp.d.ts');
console.log('Wrote dist/raw/* (separate JS + WASM)');

try {
  fs.rmSync(pkgDir, { recursive: true, force: true });
  console.log('Removed pkg/ directory after bundling');
} catch (e) {
  console.warn('Warning: failed to remove pkg/ directory:', e?.message || e);
}
