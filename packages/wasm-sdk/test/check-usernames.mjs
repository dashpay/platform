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
import init, * as wasmSdk from '../pkg/wasm_sdk.js';

// Initialize WASM
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Prefetch quorums
await wasmSdk.prefetch_trusted_quorums_testnet();

// Use trusted builder
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

try {
    const usernames = await wasmSdk.get_dpns_usernames(
        sdk,
        '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
        10
    );
    console.log('Usernames for identity:', usernames);
    
    // Also try to resolve a specific username
    if (usernames && usernames.length > 0) {
        const username = usernames[0];
        // For contested resources, we'd use the parent domain and label
        console.log(`\nFor contested resources, use:
- Parent domain: ${username.parentDomainName || 'dash'}
- Label: ${username.label}`);
    }
} catch (e) {
    console.log('Error:', e.message);
}

sdk.free();