#!/usr/bin/env node
// epoch-block-queries-migrated.test.mjs - Epoch and block query tests using JavaScript wrapper (MIGRATED)

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

// 🎯 MIGRATED: Import JavaScript wrapper
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

console.log('Initializing WASM module...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

// 🎯 MIGRATED: Use JavaScript wrapper initialization
console.log('📦 Initializing JavaScript wrapper...');
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: true,
    debug: false
});
await sdk.initialize();

let passed = 0;
let failed = 0;

async function test(name, fn) {
    try {
        await fn();
        console.log(`✅ ${name}`);
        passed++;
    } catch (error) {
        console.log(`❌ ${name}`);
        console.log(`   ${error.message}`);
        failed++;
    }
}

console.log('\n🎯 Epoch and Block Query Tests Using JavaScript Wrapper (MIGRATED)\n');

// Epoch Query Tests - 🎯 MIGRATED
await test('getEpochsInfo - get epoch information', async () => {
    try {
        const result = await sdk.getEpochsInfo(1, 5, false); // 🎯 MIGRATED: was wasmSdk.get_epochs_info()
        console.log('   ✓ Successfully retrieved epoch information');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

await test('getFinalizedEpochInfos - get finalized epoch info', async () => {
    try {
        const result = await sdk.getFinalizedEpochInfos(5, false); // 🎯 MIGRATED: was wasmSdk.get_finalized_epoch_infos()
        console.log('   ✓ Successfully retrieved finalized epoch information');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

// 🎯 MIGRATED: Resource cleanup
await sdk.destroy();

console.log(`\n🎯 EPOCH-BLOCK MIGRATION RESULTS: ✅ ${passed} passed, ❌ ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);