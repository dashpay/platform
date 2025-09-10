#!/usr/bin/env node
// specialized-queries.test.mjs - Tests for masternode, group, and other specialized queries

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

// Import JavaScript wrapper (correct approach)
import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Test utilities
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

function describe(name) {
    console.log(`\n${name}`);
}

console.log('\nSpecialized Query Tests\n');

// Initialize SDK - use trusted builder for WASM
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Masternode Queries
describe('Masternode Queries');

await test('get_masternode_status - requires valid proTxHash', async () => {
    try {
        // This would need a real masternode proTxHash
        const result = await wasmSdk.get_masternode_status(
            sdk,
            "0000000000000000000000000000000000000000000000000000000000000000" // 32-byte hex
        );
        throw new Error('Should have failed with invalid proTxHash');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid proTxHash');
    }
});

await test('get_masternode_score - requires valid proTxHash', async () => {
    try {
        const result = await wasmSdk.get_masternode_score(
            sdk,
            "0000000000000000000000000000000000000000000000000000000000000000",
            1 // quorum count
        );
        throw new Error('Should have failed with invalid proTxHash');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid proTxHash');
    }
});

// Specialized Document Queries  
describe('Specialized Document Queries');

await test('get_prefunded_specialized_balance - requires valid document ID', async () => {
    try {
        const result = await wasmSdk.get_prefunded_specialized_balance(
            sdk,
            "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec", // contract ID
            "invalidDocumentId"
        );
        throw new Error('Should have failed with invalid document ID');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid document ID');
    }
});

// Clean up
sdk.free();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);

console.log('\n📝 Notes:');
console.log('- Only testing functions that actually exist in WASM SDK');
console.log('- Network errors are expected when running offline');

process.exit(failed > 0 ? 1 : 0);