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

// Use the correct test mnemonic and IDs from wallet-lib
const mnemonic = "birth kingdom trash renew flavor utility donkey gasp regular alert pave layer";

// These are the exact IDs from wallet-lib test
const userUniqueId = '0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a';
const contactUniqueId = '0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5';

// Build the exact same path as wallet-lib
const path = `m/9'/5'/15'/0'/${userUniqueId}'/${contactUniqueId}'/0`;

console.log('\n=== Testing DIP15 Path with Correct IDs ===');
console.log('Path:', path);
console.log('Expected private key: fac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60');

try {
    const result = wasmSdk.derive_key_from_seed_with_extended_path(mnemonic, null, path, "mainnet");
    console.log('\nResult private key:  ', result.private_key_hex);
    console.log('Match:', result.private_key_hex === 'fac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60' ? '✅' : '❌');
    
    // Also check xprv
    console.log('\nResult xprv:', result.xprv);
    console.log('Expected:    xprvA7UQ3tiYkkm4TsNWx9ZrzRQ6CNL3hUibrbvcwtp9nbyMwzQp3Tbi1ygZQaPoigDhCf8XUjMmGK2NbnB2kLXPYg99Lp6e3iki318sdWcFN3q');
} catch (error) {
    console.error('Error:', error.message);
}

process.exit(0);