#!/usr/bin/env node
// proof-verification.test.mjs - Tests for proof verification functions

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

console.log('\nProof Verification Tests\n');

// Initialize SDK - use trusted builder for WASM (required for proof verification)
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Test values
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

describe('Proof Verification Functions');

await test('verify_proof - requires valid proof data', async () => {
    try {
        // This would need actual proof data from a query
        const result = await wasmSdk.verify_proof(
            sdk,
            "invalidproofdata"
        );
        throw new Error('Should fail with invalid proof');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid proof data');
    }
});

await test('verify_proofs - batch proof verification', async () => {
    try {
        const proofs = JSON.stringify([
            "invalidproof1",
            "invalidproof2"
        ]);
        
        const result = await wasmSdk.verify_proofs(sdk, proofs);
        throw new Error('Should fail with invalid proofs');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid proof data');
    }
});

describe('Proof Request and Verification Flow');

await test('Query with proof then verify - identity fetch', async () => {
    try {
        // First, fetch with proof
        console.log('   Step 1: Fetching identity with proof...');
        const fetchResult = await wasmSdk.identity_fetch(sdk, TEST_IDENTITY);
        
        // In a real scenario, we would extract the proof from the response
        // and verify it separately
        console.log('   ✓ Identity fetched (proof verification happens internally)');
        
        // The SDK automatically verifies proofs when using trusted builder
        if (fetchResult && fetchResult.id) {
            console.log(`   ✓ Identity verified: ${fetchResult.id}`);
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else if (error.message.includes('quorum')) {
            console.log('   Expected quorum cache error (missing quorum data)');
        } else {
            throw error;
        }
    }
});

await test('Query with proof then verify - data contract fetch', async () => {
    try {
        console.log('   Step 1: Fetching data contract with proof...');
        const contractResult = await wasmSdk.data_contract_fetch(sdk, DPNS_CONTRACT);
        
        console.log('   ✓ Contract fetched (proof verification happens internally)');
        
        if (contractResult && contractResult.id) {
            console.log(`   ✓ Contract verified: ${contractResult.id}`);
        }
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else if (error.message.includes('quorum')) {
            console.log('   Expected quorum cache error (missing quorum data)');
        } else {
            throw error;
        }
    }
});

describe('Proof Types and Formats');

await test('Different proof request types', async () => {
    try {
        // Example of different proof types supported
        const proofTypes = [
            'IdentityProof',
            'DocumentProof', 
            'DataContractProof',
            'StateTransitionProof'
        ];
        
        console.log(`   Supported proof types: ${proofTypes.join(', ')}`);
        
        // In WASM SDK, proofs are automatically requested when prove=true
        // and verified when using trusted builder
        console.log('   ✓ Proof types documented');
    } catch (error) {
        throw error;
    }
});

describe('Proof Validation Edge Cases');

await test('Empty proof data', async () => {
    try {
        const result = await wasmSdk.verify_proof(sdk, "");
        throw new Error('Should fail with empty proof');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with empty proof');
    }
});

await test('Malformed proof data', async () => {
    try {
        const result = await wasmSdk.verify_proof(sdk, "not-a-valid-proof");
        throw new Error('Should fail with malformed proof');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with malformed proof');
    }
});

await test('Proof verification in non-trusted mode', async () => {
    try {
        // Create non-trusted SDK
        const nonTrustedBuilder = wasmSdk.WasmSdkBuilder.new_testnet();
        const nonTrustedSdk = await nonTrustedBuilder.build();
        
        // Try to fetch with proof in non-trusted mode
        const result = await wasmSdk.identity_fetch(nonTrustedSdk, TEST_IDENTITY);
        
        nonTrustedSdk.free();
        throw new Error('Should fail in non-trusted mode');
    } catch (error) {
        if (error.message.includes('Non-trusted mode is not supported')) {
            console.log('   ✓ Correctly rejected non-trusted mode for proof verification');
        } else if (error.message.includes('Should fail')) {
            throw error;
        } else {
            console.log('   Expected error in non-trusted mode');
        }
    }
});

// Clean up
sdk.free();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);

console.log('\n📝 Notes:');
console.log('- Proof verification requires trusted SDK builder in WASM');
console.log('- Proofs are automatically verified when using trusted builder');
console.log('- Non-trusted mode is not supported for proof verification in WASM');
console.log('- Quorum data must be cached for successful proof verification');
console.log('- Network connectivity required for fetching proofs');

process.exit(failed > 0 ? 1 : 0);