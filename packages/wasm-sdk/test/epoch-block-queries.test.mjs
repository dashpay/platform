#!/usr/bin/env node
// epoch-block-queries.test.mjs - Tests for epoch and block query functions

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
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
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

console.log('\nEpoch & Block Query Tests\n');

// Test values from docs.html
const TEST_EVONODE_ID = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';

// Initialize SDK - prefetch quorums for trusted mode
console.log('Prefetching trusted quorums...');
try {
    await wasmSdk.prefetch_trusted_quorums_testnet();
    console.log('Quorums prefetched successfully');
} catch (error) {
    console.log('Warning: Could not prefetch quorums:', error.message);
}

// Use trusted builder as required for WASM
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Epoch Info Queries
describe('Epoch Information Queries');

await test('get_epochs_info - fetch epoch information', async () => {
    try {
        const result = await wasmSdk.get_epochs_info(
            sdk,
            1000,     // start epoch
            100,      // count
            true      // ascending
        );
        console.log(`   Found ${result?.length || 0} epochs`);
        if (result && result.length > 0) {
            const firstEpoch = result[0];
            console.log(`   First epoch: ${firstEpoch.epochIndex || 'N/A'}`);
            console.log(`   Start time: ${firstEpoch.startTime || 'N/A'}`);
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_finalized_epoch_infos - fetch finalized epoch infos', async () => {
    try {
        const result = await wasmSdk.get_finalized_epoch_infos(
            sdk,
            8635,     // start epoch
            100       // count
        );
        console.log(`   Found ${result?.length || 0} finalized epochs`);
        if (result && result.length > 0) {
            const firstEpoch = result[0];
            console.log(`   First epoch: ${firstEpoch.epochIndex || 'N/A'}`);
            console.log(`   Finalized: ${firstEpoch.isFinalized || false}`);
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Evonode Block Queries
describe('Evonode Block Queries');

await test('get_evonodes_proposed_epoch_blocks_by_ids - fetch blocks by IDs', async () => {
    try {
        const result = await wasmSdk.get_evonodes_proposed_epoch_blocks_by_ids(
            sdk,
            8635,                // epoch number
            [TEST_EVONODE_ID]    // evonode IDs
        );
        console.log(`   Found ${result?.length || 0} proposed blocks`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_evonodes_proposed_epoch_blocks_by_range - fetch blocks by range', async () => {
    try {
        const result = await wasmSdk.get_evonodes_proposed_epoch_blocks_by_range(
            sdk,
            TEST_EVONODE_ID,    // start after ID
            100                 // limit
        );
        console.log(`   Found ${result?.length || 0} proposed blocks in range`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Clean up
sdk.free();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);

console.log('\n📝 Notes:');
console.log('- Epochs are time periods in the Dash Platform');
console.log('- Each epoch contains multiple blocks');
console.log('- Evonodes propose blocks during consensus');
console.log('- Network errors are expected when running offline');

process.exit(failed > 0 ? 1 : 0);