#!/usr/bin/env node
// token-queries.test.mjs - Tests for token query functions

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

console.log('\nToken Query Tests\n');

// Test values from docs.html
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
const TOKEN_CONTRACT_1 = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
const TOKEN_CONTRACT_2 = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
const TOKEN_CONTRACT_3 = 'EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta';

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

// Token Query Tests
describe('Token Status and Info Queries');

await test('get_token_statuses - fetch status for multiple tokens', async () => {
    try {
        const result = await sdk.getTokenStatuses(
            sdk,
            [TOKEN_CONTRACT_1, TOKEN_CONTRACT_2]
        );
        console.log(`   Token statuses: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_token_direct_purchase_prices - get token purchase prices', async () => {
    try {
        const result = await sdk.getTokenDirectPurchasePrices(
            sdk,
            [TOKEN_CONTRACT_2]
        );
        console.log(`   Token prices: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_token_contract_info - get token contract information', async () => {
    try {
        const result = await wasmSdk.get_token_contract_info(
            sdk,
            TOKEN_CONTRACT_3
        );
        console.log(`   Token contract info retrieved`);
        if (result) {
            console.log(`   Token name: ${result.name || 'N/A'}`);
            console.log(`   Token symbol: ${result.symbol || 'N/A'}`);
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

describe('Token Supply and Distribution Queries');

await test('get_token_perpetual_distribution_last_claim - get last claim info', async () => {
    try {
        const result = await wasmSdk.get_token_perpetual_distribution_last_claim(
            sdk,
            TEST_IDENTITY,
            TOKEN_CONTRACT_3
        );
        console.log(`   Last claim info: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_token_total_supply - get token total supply', async () => {
    try {
        const result = await sdk.getTokenTotalSupply(
            sdk,
            TOKEN_CONTRACT_1
        );
        console.log(`   Total supply: ${result}`);
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
console.log('- Token contracts on testnet may have limited activity');
console.log('- Token statuses track issuance and burning states');
console.log('- Direct purchase prices are for tokens that can be bought directly');
console.log('- Network errors are expected when running offline');

process.exit(failed > 0 ? 1 : 0);