// test-setup.js - Setup for running WASM SDK tests in Node.js
const fs = require('fs');
const path = require('path');

// Polyfill for TextEncoder/TextDecoder if needed
if (typeof global.TextEncoder === 'undefined') {
    const { TextEncoder, TextDecoder } = require('util');
    global.TextEncoder = TextEncoder;
    global.TextDecoder = TextDecoder;
}

// Polyfill for crypto.getRandomValues
if (typeof global.crypto === 'undefined') {
    global.crypto = require('crypto').webcrypto;
}

// Since the WASM SDK is built for web target, we need to provide browser-like globals
global.self = global;

// Load WASM module using eval to bypass module restrictions
async function loadWasmModule() {
    // Read the JS file and modify it for Node.js
    const jsPath = path.join(__dirname, '../pkg/wasm_sdk.js');
    let jsContent = fs.readFileSync(jsPath, 'utf8');
    
    // Replace import.meta.url with a file URL
    const fileUrl = `file://${jsPath}`;
    jsContent = jsContent.replace(/import\.meta\.url/g, `'${fileUrl}'`);
    
    // Create a module wrapper
    const moduleWrapper = `
        const exports = {};
        const module = { exports };
        ${jsContent}
        return module.exports;
    `;
    
    // Evaluate the module
    const wasmModule = eval(`(async function() { ${moduleWrapper} })()`);
    const loadedModule = await wasmModule;
    
    // Read and initialize WASM
    const wasmPath = path.join(__dirname, '../pkg/wasm_sdk_bg.wasm');
    const wasmBuffer = fs.readFileSync(wasmPath);
    
    // Initialize the module
    await loadedModule.default(wasmBuffer);
    
    return loadedModule;
}

module.exports = { loadWasmModule };