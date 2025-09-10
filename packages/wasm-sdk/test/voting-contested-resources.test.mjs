#!/usr/bin/env node
// voting-contested-resources.test.mjs - Tests for voting and contested resources queries

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

console.log('\nVoting & Contested Resources Query Tests\n');

// Test values from docs.html
const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
// Username for TEST_IDENTITY is "therealslimshaddy5.dash"
const TEST_PARENT_DOMAIN = 'dash';
const TEST_LABEL = 'therealslimshaddy5';

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

// Contested Resources Tests
describe('Contested Resources Queries');

await test('get_contested_resources - fetch contested domain names', async () => {
    try {
        // Based on Rust signature, we need all parameters
        const result = await wasmSdk.get_contested_resources(
            sdk,
            'domain',                           // document_type_name
            DPNS_CONTRACT,                      // data_contract_id
            'parentNameAndLabel',               // index_name
            'documents',                        // result_type
            null,                               // allow_include_locked_and_abstaining_vote_tally
            null,                               // start_at_value
            100,                                // limit
            null,                               // offset
            true                                // order_ascending
        );
        console.log(`   Found ${result?.contestedResources?.length || 0} contested resources`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_contested_resource_vote_state - get vote state for contested resource', async () => {
    try {
        const result = await wasmSdk.get_contested_resource_vote_state(
            sdk,
            DPNS_CONTRACT,                      // data_contract_id
            'domain',                           // document_type_name
            'parentNameAndLabel',               // index_name
            [TEST_PARENT_DOMAIN, TEST_LABEL],   // index_values: [parent domain, label]
            'documentTypeName',                 // result_type
            null,                               // allow_include_locked_and_abstaining_vote_tally
            null,                               // start_at_identifier_info
            100,                                // count
            true                                // order_ascending
        );
        console.log(`   Vote state retrieved`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_contested_resource_voters_for_identity - get voters for identity', async () => {
    try {
        const result = await wasmSdk.get_contested_resource_voters_for_identity(
            sdk,
            DPNS_CONTRACT,                      // contract_id
            'domain',                           // document_type_name
            'parentNameAndLabel',               // index_name
            [TEST_PARENT_DOMAIN, TEST_LABEL],   // index_values: [parent domain, label]
            TEST_IDENTITY,                      // contestant_id
            null,                               // start_at_voter_info
            100,                                // limit
            true                                // order_ascending
        );
        console.log(`   Found ${result?.voters?.length || 0} voters`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_contested_resource_identity_votes - get votes by identity', async () => {
    try {
        const result = await wasmSdk.get_contested_resource_identity_votes(
            sdk,
            TEST_IDENTITY      // identity ID
        );
        console.log(`   Found ${result?.length || 0} votes by identity`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_vote_polls_by_end_date - get vote polls in date range', async () => {
    try {
        // Function expects string timestamps based on Rust signature
        const endTime = Date.now();
        const startTime = endTime - 86400000; // 24 hours ago
        
        const result = await wasmSdk.get_vote_polls_by_end_date(
            sdk,
            startTime.toString(),
            endTime.toString(),
            100,    // limit
            true    // order_ascending
        );
        console.log(`   Found ${result?.votePolls?.length || 0} vote polls in range`);
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
console.log('- Contested resources are typically domain names that multiple users want');
console.log('- Vote polls track masternode voting on contested resources');
console.log('- Network errors are expected when running offline');

process.exit(failed > 0 ? 1 : 0);