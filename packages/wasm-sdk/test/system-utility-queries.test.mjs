#!/usr/bin/env node
// system-utility-queries.test.mjs - Tests for system and utility query functions

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
import init, * as wasmSdk from '../pkg/dash_wasm_sdk.js';
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

console.log('\nSystem & Utility Query Tests\n');

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

// System Information Queries
describe('System Information Queries');

await test('get_current_quorums_info - fetch current quorum information', async () => {
    try {
        const result = await wasmSdk.get_current_quorums_info(sdk);
        console.log(`   Current quorums info retrieved`);
        if (result) {
            console.log(`   Number of quorums: ${result.length || 0}`);
            if (result.length > 0) {
                const firstQuorum = result[0];
                console.log(`   First quorum type: ${firstQuorum.quorumType || 'N/A'}`);
                console.log(`   Member count: ${firstQuorum.memberCount || 'N/A'}`);
            }
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_total_credits_in_platform - fetch total platform credits', async () => {
    try {
        const result = await wasmSdk.get_total_credits_in_platform(sdk);
        console.log(`   Total credits in platform: ${result}`);
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
console.log('- Current quorums info shows active masternode quorums');
console.log('- Total credits represents all credits in the platform');
console.log('- Network errors are expected when running offline');

process.exit(failed > 0 ? 1 : 0);