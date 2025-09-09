#!/usr/bin/env node
// protocol-version-queries.test.mjs - Tests for protocol and version query functions

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

console.log('\nProtocol & Version Query Tests\n');

// Test values from docs.html
const TEST_PROTX_HASH = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';

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

// Protocol Version Queries
describe('Protocol Version Upgrade Queries');

await test('get_protocol_version_upgrade_state - fetch upgrade state', async () => {
    try {
        const result = await wasmSdk.get_protocol_version_upgrade_state(sdk);
        console.log(`   Protocol version upgrade state retrieved`);
        if (result) {
            console.log(`   Current version: ${result.currentVersion || 'N/A'}`);
            console.log(`   Next version: ${result.nextVersion || 'N/A'}`);
            console.log(`   Vote status: ${result.voteStatus || 'N/A'}`);
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_protocol_version_upgrade_vote_status - fetch vote status', async () => {
    try {
        const result = await wasmSdk.get_protocol_version_upgrade_vote_status(
            sdk,
            TEST_PROTX_HASH,    // start protx hash
            100                 // count
        );
        console.log(`   Found ${result?.length || 0} vote statuses`);
        if (result && result.length > 0) {
            const firstVote = result[0];
            console.log(`   First vote from: ${firstVote.proTxHash || 'N/A'}`);
            console.log(`   Vote: ${firstVote.vote || 'N/A'}`);
        }
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
console.log('- Protocol version upgrades require masternode voting');
console.log('- Upgrade state shows current and proposed protocol versions');
console.log('- Vote status shows how individual masternodes voted');
console.log('- Network errors are expected when running offline');

process.exit(failed > 0 ? 1 : 0);