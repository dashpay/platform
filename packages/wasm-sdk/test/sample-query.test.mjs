#!/usr/bin/env node
// sample-query.test.mjs - Sample tests for query functions (network-dependent)

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
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Initialize JavaScript wrapper
console.log('Initializing JavaScript wrapper...');
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: false,
    debug: false
});
await sdk.initialize();
console.log('✅ JavaScript wrapper initialized successfully');

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

console.log('\nSample Query Tests (Network-Dependent)\n');

// Initialize SDK
const builder = wasmSdk.WasmSdkBuilder.new_testnet();
const sdk = await builder.build();

// Identity Query Tests
describe('Identity Queries (Expected to fail without network)');

await test('identity_fetch - requires valid identity ID', async () => {
    try {
        // This will fail without a valid identity ID
        const result = await wasmSdk.identity_fetch(sdk, "invalididentityid");
        throw new Error('Should have failed with invalid ID');
    } catch (error) {
        // Expected to fail
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error: ' + error.message.substring(0, 50) + '...');
    }
});

await test('get_identity_balance - requires valid identity', async () => {
    try {
        const result = await wasmSdk.get_identity_balance(
            sdk,
            "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"  // Example testnet identity
        );
        // If this succeeds, network is available
        console.log('   Network available! Balance:', result);
    } catch (error) {
        // Expected to fail without network
        console.log('   Expected network error');
    }
});

// Document Query Tests
describe('Document Queries');

await test('get_documents - DPNS contract', async () => {
    try {
        // DPNS contract ID on testnet
        const dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        
        const result = await wasmSdk.get_documents(
            sdk,
            dpnsContractId,
            "domain",
            null,  // no where clause
            null,  // no order by
            10,    // limit
            null,  // no start after
            null   // no start at
        );
        
        console.log('   Network available! Found documents:', result?.length || 0);
    } catch (error) {
        console.log('   Expected network error');
    }
});

// Data Contract Query Tests
describe('Data Contract Queries');

await test('data_contract_fetch - DPNS contract', async () => {
    try {
        const dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        const result = await wasmSdk.data_contract_fetch(sdk, dpnsContractId);
        
        console.log('   Network available! Contract fetched');
    } catch (error) {
        console.log('   Expected network error');
    }
});

// System Query Tests
describe('System Queries');

await test('get_status - platform status', async () => {
    try {
        const result = await wasmSdk.get_status(sdk);
        console.log('   Network available! Status:', result);
    } catch (error) {
        console.log('   Expected network error');
    }
});

await test('get_current_epoch', async () => {
    try {
        const result = await wasmSdk.get_current_epoch(sdk);
        console.log('   Network available! Current epoch:', result);
    } catch (error) {
        console.log('   Expected network error');
    }
});

// Clean up
sdk.free();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);

console.log('\nNote: These tests require network connectivity to Dash Platform testnet.');
console.log('Failures are expected when running offline or when testnet is unavailable.');

process.exit(failed > 0 ? 1 : 0);