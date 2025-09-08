#!/usr/bin/env node
// system-utility-queries-migrated.test.mjs - System utility tests using JavaScript wrapper (MIGRATED)

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

console.log('\n🎯 System Utility Query Tests Using JavaScript Wrapper (MIGRATED)\n');

await test('getCurrentQuorumsInfo - get current quorum information', async () => {
    try {
        const result = await sdk.getCurrentQuorumsInfo(); // 🎯 MIGRATED: was wasmSdk.get_current_quorums_info()
        console.log('   ✓ Successfully retrieved current quorums information');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

await test('getTotalCreditsInPlatform - get total credits', async () => {
    try {
        const result = await sdk.getTotalCreditsInPlatform(); // 🎯 MIGRATED: was wasmSdk.get_total_credits_in_platform()
        console.log('   ✓ Successfully retrieved total credits in platform');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

await sdk.destroy();

console.log(`\n🎯 SYSTEM-UTILITY MIGRATION RESULTS: ✅ ${passed} passed, ❌ ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);