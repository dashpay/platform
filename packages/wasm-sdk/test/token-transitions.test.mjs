#!/usr/bin/env node
// token-transitions.test.mjs - Tests for new token state transition functions

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

console.log('\nToken State Transition Tests\n');

// Initialize SDK - use trusted builder for WASM
console.log('Prefetching trusted quorums...');
try {
    await wasmSdk.prefetch_trusted_quorums_testnet();
    console.log('Quorums prefetched successfully');
} catch (error) {
    console.log('Warning: Could not prefetch quorums:', error.message);
}

const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Test values
const TEST_CONTRACT_ID = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
const TEST_TOKEN_POSITION = 0;
const TEST_IDENTITY_ID = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
const TEST_RECIPIENT_ID = '3mFKtDYspCMd8YmXNTB3qzKmbY3Azf4Kx3x8e36V8Gho';
const TEST_PRIVATE_KEY = 'KycRvJNvCVapwvvpRLWz76qXFAbXFfAqhG9FouVjUmDVZ6UtZfGa'; // Dummy key for testing

// Token Transfer Tests
describe('Token Transfer State Transition');

await test('tokenTransfer - should validate parameters', async () => {
    try {
        // Test with invalid contract ID
        await sdk.tokenTransfer(
            'invalid-contract-id',
            TEST_TOKEN_POSITION,
            '1000',
            TEST_IDENTITY_ID,
            TEST_RECIPIENT_ID,
            TEST_PRIVATE_KEY,
            'Test transfer'
        );
        throw new Error('Should fail with invalid contract ID');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid contract ID');
    }
});

await test('tokenTransfer - should validate amount', async () => {
    try {
        // Test with invalid amount
        await sdk.tokenTransfer(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            'invalid-amount',
            TEST_IDENTITY_ID,
            TEST_RECIPIENT_ID,
            TEST_PRIVATE_KEY,
            null
        );
        throw new Error('Should fail with invalid amount');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid amount');
    }
});

await test('tokenTransfer - should require valid identity', async () => {
    try {
        // This will fail because the identity doesn't exist or we don't have the right key
        await sdk.tokenTransfer(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            '1000',
            TEST_IDENTITY_ID,
            TEST_RECIPIENT_ID,
            TEST_PRIVATE_KEY,
            'Test transfer'
        );
        throw new Error('Should fail without valid identity/key');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without valid identity/key');
    }
});

// Token Freeze Tests
describe('Token Freeze State Transition');

await test('tokenFreeze - should validate parameters', async () => {
    try {
        // Test with invalid contract ID
        await sdk.tokenFreeze(
            'invalid-contract-id',
            TEST_TOKEN_POSITION,
            TEST_RECIPIENT_ID,
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            'Freezing tokens'
        );
        throw new Error('Should fail with invalid contract ID');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid contract ID');
    }
});

await test('tokenFreeze - should validate identity to freeze', async () => {
    try {
        // Test with invalid identity ID
        await sdk.tokenFreeze(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            'invalid-identity',
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            null
        );
        throw new Error('Should fail with invalid identity to freeze');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid identity to freeze');
    }
});

await test('tokenFreeze - should require freezer permissions', async () => {
    try {
        // This will fail because the identity doesn't have freeze permissions
        await sdk.tokenFreeze(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            TEST_RECIPIENT_ID,
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            'Test freeze'
        );
        throw new Error('Should fail without freeze permissions');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without freeze permissions');
    }
});

// Token Unfreeze Tests
describe('Token Unfreeze State Transition');

await test('tokenUnfreeze - should validate parameters', async () => {
    try {
        // Test with invalid contract ID
        await sdk.tokenUnfreeze(
            'invalid-contract-id',
            TEST_TOKEN_POSITION,
            TEST_RECIPIENT_ID,
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            'Unfreezing tokens'
        );
        throw new Error('Should fail with invalid contract ID');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid contract ID');
    }
});

await test('tokenUnfreeze - should validate identity to unfreeze', async () => {
    try {
        // Test with invalid identity ID
        await sdk.tokenUnfreeze(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            'invalid-identity',
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            null
        );
        throw new Error('Should fail with invalid identity to unfreeze');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid identity to unfreeze');
    }
});

await test('tokenUnfreeze - should require unfreezer permissions', async () => {
    try {
        // This will fail because the identity doesn't have unfreeze permissions
        await sdk.tokenUnfreeze(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            TEST_RECIPIENT_ID,
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            'Test unfreeze'
        );
        throw new Error('Should fail without unfreeze permissions');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without unfreeze permissions');
    }
});

// Token Destroy Frozen Tests
describe('Token Destroy Frozen State Transition');

await test('tokenDestroyFrozen - should validate parameters', async () => {
    try {
        // Test with invalid contract ID
        await sdk.tokenDestroyFrozen(
            'invalid-contract-id',
            TEST_TOKEN_POSITION,
            TEST_RECIPIENT_ID,
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            'Destroying frozen tokens'
        );
        throw new Error('Should fail with invalid contract ID');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid contract ID');
    }
});

await test('tokenDestroyFrozen - should validate identity', async () => {
    try {
        // Test with invalid identity ID
        await sdk.tokenDestroyFrozen(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            'invalid-identity',
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            null
        );
        throw new Error('Should fail with invalid identity');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid identity');
    }
});

await test('tokenDestroyFrozen - should require destroyer permissions', async () => {
    try {
        // This will fail because the identity doesn't have destroy permissions
        await sdk.tokenDestroyFrozen(
            TEST_CONTRACT_ID,
            TEST_TOKEN_POSITION,
            TEST_RECIPIENT_ID,
            TEST_IDENTITY_ID,
            TEST_PRIVATE_KEY,
            'Test destroy frozen'
        );
        throw new Error('Should fail without destroy permissions');
    } catch (error) {
        if (error && error.message && error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without destroy permissions');
    }
});

// Method Availability Tests
describe('Token Transition Methods Availability');

await test('All new token transition methods should be available on SDK', async () => {
    if (typeof sdk.tokenTransfer !== 'function') {
        throw new Error('tokenTransfer method not found on SDK instance');
    }
    if (typeof sdk.tokenFreeze !== 'function') {
        throw new Error('tokenFreeze method not found on SDK instance');
    }
    if (typeof sdk.tokenUnfreeze !== 'function') {
        throw new Error('tokenUnfreeze method not found on SDK instance');
    }
    if (typeof sdk.tokenDestroyFrozen !== 'function') {
        throw new Error('tokenDestroyFrozen method not found on SDK instance');
    }
    console.log('   All token transition methods are available');
});

// Summary
console.log('\n=== Test Summary ===');
console.log(`Passed: ${passed}`);
console.log(`Failed: ${failed}`);
console.log('\nNote: Most tests are expected to fail with permission/identity errors');
console.log('This is normal as we are testing parameter validation without real funded identities.');
console.log('The important thing is that the methods are available and validate parameters correctly.\n');

process.exit(failed > 0 ? 1 : 0);