#!/usr/bin/env node

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Get directory paths
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Set up globals for WASM
if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

// Import WASM SDK
import init, * as wasmSdk from './pkg/wasm_sdk.js';

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, './pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Test both paths
const mnemonic = "birth kingdom trash renew flavor utility donkey gasp regular alert pave layer";

// Path 1: Just the 256-bit indices (from test_dip14_vector.mjs)
const path1 = "m/0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a/0xf537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6'";

// Path 2: Full DIP15 path (from test_dip14_implementation.mjs) 
const path2 = "m/9'/5'/15'/0'/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0";

console.log('\n=== Testing Path 1 (Short) ===');
console.log('Path:', path1);
try {
    const result1 = wasmSdk.derive_key_from_seed_with_extended_path(mnemonic, null, path1, "testnet");
    console.log('Private key:', result1.private_key_hex);
} catch (error) {
    console.error('Error:', error.message);
}

console.log('\n=== Testing Path 2 (Full DIP15) ===');
console.log('Path:', path2);
try {
    const result2 = wasmSdk.derive_key_from_seed_with_extended_path(mnemonic, null, path2, "testnet");
    console.log('Private key:', result2.private_key_hex);
    console.log('Expected:    fac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60');
    console.log('Match:', result2.private_key_hex === 'fac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60' ? '✅' : '❌');
} catch (error) {
    console.error('Error:', error.message);
}

process.exit(0);