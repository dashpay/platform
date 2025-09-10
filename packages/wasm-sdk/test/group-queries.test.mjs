#!/usr/bin/env node
// group-queries.test.mjs - Tests for group query functions

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

// Pre-load WASM for Node.js compatibility
console.log('Initializing WASM module...');
const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

// Initialize JavaScript wrapper
console.log('Initializing JavaScript wrapper...');
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: true,
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

console.log('\nGroup Query Tests\n');

// Test values from docs.html
const TEST_GROUP_ID = '49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N';
const TEST_ACTION_ID = '6XJzL6Qb8Zhwxt4HFwh8NAn7q1u4dwdoUf8EmgzDudFZ';

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

// Group Info Queries
describe('Group Information Queries');

await test('get_group_info - fetch specific group info', async () => {
    try {
        const result = await sdk.getGroupInfo(
            sdk,
            TEST_GROUP_ID,    // group ID
            0                 // subgroup position
        );
        console.log(`   Group info retrieved`);
        if (result) {
            console.log(`   Group type: ${result.type || 'N/A'}`);
            console.log(`   Member count: ${result.memberCount || 'N/A'}`);
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_group_infos - fetch multiple group infos', async () => {
    try {
        const result = await sdk.getGroupInfos(
            sdk,
            TEST_GROUP_ID,    // group ID
            null,             // start after
            100               // limit
        );
        console.log(`   Found ${result?.length || 0} group infos`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Group Action Queries
describe('Group Action Queries');

await test('get_group_actions - fetch group actions', async () => {
    try {
        const result = await wasmSdk.get_group_actions(
            sdk,
            TEST_GROUP_ID,    // group ID
            0,                // subgroup position
            'ACTIVE',         // action status
            null,             // start after
            100               // limit
        );
        console.log(`   Found ${result?.length || 0} group actions`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_group_action_signers - fetch action signers', async () => {
    try {
        const result = await wasmSdk.get_group_action_signers(
            sdk,
            TEST_GROUP_ID,    // group ID
            0,                // subgroup position
            'ACTIVE',         // action status
            TEST_ACTION_ID    // action ID
        );
        console.log(`   Found ${result?.length || 0} action signers`);
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
console.log('- Groups are collections of identities that can perform collective actions');
console.log('- Group actions require signatures from multiple group members');
console.log('- Action status can be ACTIVE, COMPLETED, or CANCELLED');
console.log('- Network errors are expected when running offline');

process.exit(failed > 0 ? 1 : 0);