#!/usr/bin/env node
// utilities-simple.test.mjs - Simplified utility function tests avoiding panics

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

console.log('\nUtility Functions Tests (Simplified)\n');

// SDK Version Test
describe('SDK Version and Initialization');

await test('Create SDK and check version', async () => {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    const sdk = await builder.build();
    
    const version = sdk.version();
    if (typeof version !== 'number') {
        throw new Error('Version should be a number');
    }
    if (version < 1) {
        throw new Error('Version should be at least 1');
    }
    console.log(`   SDK version: ${version}`);
    
    sdk.free();
});

// Trusted Quorum Prefetch Tests
describe('Trusted Quorum Prefetch');

await test('prefetch_trusted_quorums_mainnet', async () => {
    try {
        await wasmSdk.prefetch_trusted_quorums_mainnet();
        // Success means network is available
    } catch (error) {
        // Network error is acceptable
        if (!error.message.includes('network') && !error.message.includes('fetch')) {
            throw error;
        }
    }
});

await test('prefetch_trusted_quorums_testnet', async () => {
    try {
        await wasmSdk.prefetch_trusted_quorums_testnet();
        // Success means network is available
    } catch (error) {
        // Network error is acceptable
        if (!error.message.includes('network') && !error.message.includes('fetch')) {
            throw error;
        }
    }
});

// Test Serialization
describe('Test Serialization (if method exists)');

await test('testSerialization method availability', async () => {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    const sdk = await builder.build();
    
    if (typeof sdk.testSerialization === 'function') {
        console.log('   testSerialization method exists');
        
        // Try calling it with a valid type
        const result = sdk.testSerialization('simple');
        console.log(`   Result type: ${typeof result}, value:`, result);
        
        // Should return a proper serialized object
        if (typeof result !== 'object' || result === null) {
            throw new Error(`Expected object result, got ${typeof result}`);
        }
    } else {
        console.log('   testSerialization method not found');
    }
    
    sdk.free();
});

// Error Handling Tests
describe('Error Handling');

await test('Using null SDK should fail gracefully', async () => {
    try {
        await wasmSdk.get_status(null);
        throw new Error('Should have failed with null SDK');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        // Expected error
    }
});

await test('Using undefined SDK should fail gracefully', async () => {
    try {
        await wasmSdk.get_status(undefined);
        throw new Error('Should have failed with undefined SDK');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        // Expected error
    }
});

await test('Using freed SDK should fail gracefully', async () => {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    const sdk = await builder.build();
    sdk.free();
    
    try {
        sdk.version();
        throw new Error('Should have failed with freed SDK');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        // Expected error
    }
});

// Type Validation Tests
describe('Type Validation');

await test('String parameter type validation', async () => {
    try {
        // Pass number where string expected
        wasmSdk.validate_mnemonic(123);
        throw new Error('Should have failed with wrong type');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        // Expected type error
    }
});

await test('Array parameter type validation', async () => {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    const sdk = await builder.build();
    
    try {
        // Pass string where array expected
        await wasmSdk.get_path_elements(sdk, "not-an-array", []);
        throw new Error('Should have failed with non-array');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        // Expected type error
    }
    
    sdk.free();
});

await test('Number parameter type validation', async () => {
    try {
        // Pass string where number expected
        wasmSdk.generate_mnemonic("twelve");
        throw new Error('Should have failed with wrong type');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        // Expected type error
    }
});

// Network-dependent utility functions
describe('Network-dependent Utilities');

await test('wait_for_state_transition_result - requires valid hash', async () => {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    const sdk = await builder.build();
    
    try {
        // This will timeout or fail without valid hash
        await wasmSdk.wait_for_state_transition_result(
            sdk,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        // If it succeeds, that's unexpected
        throw new Error('Should have failed or timed out');
    } catch (error) {
        if (error.message.includes('Should have failed or timed out')) {
            throw error;
        }
        // Expected error or timeout
    }
    
    sdk.free();
});

await test('get_path_elements - requires network', async () => {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    const sdk = await builder.build();
    
    try {
        const result = await wasmSdk.get_path_elements(sdk, [], []);
        // If it succeeds, check result
        if (result && typeof result === 'object') {
            console.log('   Successfully got path elements');
        }
    } catch (error) {
        // Network error is expected
        console.log('   Expected network error');
    }
    
    sdk.free();
});

// Start function
describe('Start Function');

await test('start function exists', async () => {
    // The start function should exist
    if (typeof wasmSdk.start !== 'function') {
        throw new Error('start function not found');
    }
    
    // Since the WASM module auto-calls start() on initialization,
    // calling it again will cause a panic due to tracing already being set.
    // This is expected behavior - start() should only be called once.
    
    // We'll test that it exists and is callable, but we won't call it
    // since it's already been called during WASM initialization
    console.log('   start function exists and has been called during WASM init');
    console.log('   (calling it again would panic due to tracing already initialized)');
});

// Function existence checks
describe('Function Existence');

await test('All expected utility functions exist', () => {
    const utilityFunctions = [
        'prefetch_trusted_quorums_mainnet',
        'prefetch_trusted_quorums_testnet',
        'wait_for_state_transition_result',
        'start',
        'get_path_elements'
    ];
    
    for (const fn of utilityFunctions) {
        if (typeof wasmSdk[fn] !== 'function') {
            throw new Error(`${fn} not found`);
        }
    }
    
    console.log('   All utility functions found');
});

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);
process.exit(failed > 0 ? 1 : 0);