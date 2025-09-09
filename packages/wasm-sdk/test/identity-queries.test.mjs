#!/usr/bin/env node
// identity-queries.test.mjs - Tests for identity query functions using documented testnet values

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

// Pre-load WASM for Node.js compatibility
console.log('Initializing WASM module...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
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

console.log('\nIdentity Query Tests Using Documented Testnet Values\n');

// DOCUMENTED TEST VALUES FROM docs.html
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk'; // Known testnet identity
const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'; // DPNS contract ID
const TOKEN_CONTRACT = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv'; // Example token contract

console.log('Test Values:');
console.log(`- Identity: ${TEST_IDENTITY}`);
console.log(`- DPNS Contract: ${DPNS_CONTRACT}`);
console.log(`- Token Contract: ${TOKEN_CONTRACT}`);

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

// Identity Query Tests
describe('Basic Identity Queries');

await test('identity_fetch - documented test identity', async () => {
    try {
        const result = await sdk.getIdentity(TEST_IDENTITY);
        console.log('   ✓ Identity fetched successfully');
        console.log(`   ID: ${result?.id || 'N/A'}`);
        console.log(`   Balance: ${result?.balance || 'N/A'}`);
        console.log(`   Public Keys: ${result?.publicKeys?.length || 0}`);
    } catch (error) {
        // Network error is expected if offline
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identity_balance - documented test identity', async () => {
    try {
        const result = await sdk.getIdentityBalance(TEST_IDENTITY);
        console.log(`   Balance: ${result}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identity_keys - all keys', async () => {
    try {
        const result = await wasmSdk.get_identity_keys(sdk, TEST_IDENTITY, 'all');
        console.log(`   Found ${result?.length || 0} keys`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identity_nonce', async () => {
    try {
        const result = await wasmSdk.get_identity_nonce(sdk, TEST_IDENTITY);
        console.log(`   Nonce: ${result}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identity_contract_nonce - with DPNS contract', async () => {
    try {
        const result = await wasmSdk.get_identity_contract_nonce(sdk, TEST_IDENTITY, DPNS_CONTRACT);
        console.log(`   Contract nonce: ${result}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Batch Identity Queries
describe('Batch Identity Queries');

await test('get_identities_balances - single identity', async () => {
    try {
        const result = await wasmSdk.get_identities_balances(sdk, [TEST_IDENTITY]);
        console.log(`   Balances: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identity_balance_and_revision', async () => {
    try {
        const result = await wasmSdk.get_identity_balance_and_revision(sdk, TEST_IDENTITY);
        console.log(`   Balance: ${result?.balance}, Revision: ${result?.revision}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Contract Keys
describe('Contract Keys Queries');

await test('get_identities_contract_keys - DPNS contract', async () => {
    try {
        const result = await wasmSdk.get_identities_contract_keys(
            sdk, 
            [TEST_IDENTITY], 
            DPNS_CONTRACT,
            'domain',  // document type
            'all'      // purposes
        );
        console.log(`   Found contract keys: ${result?.length || 0}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Token-related Identity Queries
describe('Token Balance Queries');

await test('get_identity_token_balances', async () => {
    try {
        const result = await wasmSdk.get_identity_token_balances(
            sdk,
            TEST_IDENTITY,
            [TOKEN_CONTRACT]
        );
        console.log(`   Token balances: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identities_token_balances', async () => {
    try {
        const result = await wasmSdk.get_identities_token_balances(
            sdk,
            [TEST_IDENTITY],
            TOKEN_CONTRACT
        );
        console.log(`   Token balances: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identity_token_infos', async () => {
    try {
        const result = await wasmSdk.get_identity_token_infos(
            sdk,
            TEST_IDENTITY,
            [TOKEN_CONTRACT]
        );
        console.log(`   Token infos: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_identities_token_infos', async () => {
    try {
        const result = await wasmSdk.get_identities_token_infos(
            sdk,
            [TEST_IDENTITY],
            TOKEN_CONTRACT
        );
        console.log(`   Token infos for identities: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Public Key Hash Queries
describe('Public Key Hash Queries');

await test('get_identity_by_public_key_hash - requires valid hash', async () => {
    try {
        // This would need a real public key hash from the test identity
        const result = await wasmSdk.get_identity_by_public_key_hash(
            sdk,
            "invalidhash" // Would need real hash
        );
        throw new Error('Should have failed with invalid hash');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid hash');
    }
});

await test('get_identity_by_non_unique_public_key_hash - requires valid hash', async () => {
    try {
        // Example non-unique public key hash from docs
        const result = await wasmSdk.get_identity_by_non_unique_public_key_hash(
            sdk,
            '518038dc858461bcee90478fd994bba8057b7531',
            null  // start_after parameter (optional)
        );
        console.log(`   Found ${result?.length || 0} identities with this public key hash`);
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
console.log('- These tests use the documented testnet values from docs.html');
console.log('- Network errors are expected when running offline');
console.log('- Some queries may timeout if testnet is slow');
console.log(`- Test identity ${TEST_IDENTITY} should have activity on testnet`);

// Cleanup
await sdk.destroy();

process.exit(failed > 0 ? 1 : 0);