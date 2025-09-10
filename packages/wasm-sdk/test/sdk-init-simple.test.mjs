#!/usr/bin/env node
// sdk-init-simple.test.mjs - Simplified SDK initialization tests

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

// Initialize JavaScript wrapper
console.log('Initializing JavaScript wrapper...');
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: false,
    debug: false
});
await sdk.initialize();
console.log('✅ JavaScript wrapper initialized successfully');

// Test results
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

console.log('\nSDK Initialization Tests\n');

// Test 1: Check WasmSdkBuilder exists
await test('WasmSdkBuilder class exists', () => {
    if (!wasmSdk.WasmSdkBuilder) {
        throw new Error('WasmSdkBuilder not found');
    }
});

// Test 2: Check static methods exist
await test('WasmSdkBuilder has static methods', () => {
    if (typeof wasmSdk.WasmSdkBuilder.new_mainnet !== 'function') {
        throw new Error('new_mainnet method not found');
    }
    if (typeof wasmSdk.WasmSdkBuilder.new_testnet !== 'function') {
        throw new Error('new_testnet method not found');
    }
    if (typeof wasmSdk.WasmSdkBuilder.getLatestVersionNumber !== 'function') {
        throw new Error('getLatestVersionNumber method not found');
    }
});

// Test 3: Get latest version
await test('getLatestVersionNumber returns a number', () => {
    const version = wasmSdk.WasmSdkBuilder.getLatestVersionNumber();
    if (typeof version !== 'number') {
        throw new Error(`Expected number, got ${typeof version}`);
    }
    console.log(`   Latest version: ${version}`);
});

// Test 4: Create and test SDK instance
await test('Can create SDK instance', async () => {
    const builder = wasmSdk.WasmSdkBuilder.new_testnet();
    const sdk = await builder.build();
    
    if (!sdk || !sdk.__wbg_ptr) {
        throw new Error('Failed to create SDK instance');
    }
    
    const version = sdk.version();
    console.log(`   SDK version: ${version}`);
    
    // Test that state transition methods exist
    if (typeof sdk.tokenMint !== 'function') {
        throw new Error('tokenMint method not found on SDK instance');
    }
    if (typeof sdk.documentCreate !== 'function') {
        throw new Error('documentCreate method not found on SDK instance');
    }
});

// Test 5: Check query functions
await test('Query functions exist as top-level exports', () => {
    const queryFunctions = [
        'identity_fetch',
        'get_documents',
        'data_contract_fetch',
        'get_status',
        'get_current_epoch'
    ];
    
    for (const fn of queryFunctions) {
        if (typeof wasmSdk[fn] !== 'function') {
            throw new Error(`${fn} not found`);
        }
    }
});

// Test 6: Check key generation functions
await test('Key generation functions exist', () => {
    const keyFunctions = [
        'generate_mnemonic',
        'validate_mnemonic',
        'mnemonic_to_seed',
        'derive_key_from_seed_with_path',
        'generate_key_pair',
        'pubkey_to_address',
        'validate_address'
    ];
    
    for (const fn of keyFunctions) {
        if (typeof wasmSdk[fn] !== 'function') {
            throw new Error(`${fn} not found`);
        }
    }
});

// Test 7: Check DPNS functions
await test('DPNS functions exist', () => {
    const dpnsFunctions = [
        'dpns_convert_to_homograph_safe',
        'dpns_is_valid_username',
        'dpns_is_contested_username',
        'dpns_register_name',
        'dpns_is_name_available',
        'dpns_resolve_name'
    ];
    
    for (const fn of dpnsFunctions) {
        if (typeof wasmSdk[fn] !== 'function') {
            throw new Error(`${fn} not found`);
        }
    }
});

// Test 8: Test mnemonic generation
await test('Can generate mnemonic', () => {
    const mnemonic = wasmSdk.generate_mnemonic(12);
    const words = mnemonic.split(' ');
    
    if (words.length !== 12) {
        throw new Error(`Expected 12 words, got ${words.length}`);
    }
    
    if (!wasmSdk.validate_mnemonic(mnemonic)) {
        throw new Error('Generated mnemonic is invalid');
    }
});

// Test 9: Test key derivation
await test('Can derive keys from mnemonic', () => {
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const result = wasmSdk.derive_key_from_seed_with_path(
        testMnemonic,
        undefined,
        "m/44'/5'/0'/0/0",
        "mainnet"
    );
    
    if (!result.address) {
        throw new Error('No address in result');
    }
    if (!result.private_key_wif) {
        throw new Error('No private key in result');
    }
    if (!result.public_key) {
        throw new Error('No public key in result');
    }
});

// Test 10: Test address validation
await test('Can validate addresses', () => {
    // Use real valid Dash addresses
    const mainnetAddress = "XoJA8qE3N2Y3jMLEtZ3vcN42qseZ8LvFf5";  // Real mainnet address
    const testnetAddress = "yRd4FhXfVGHXpsuZXPNkMrfD9GVj46pnjt";  // Real testnet address
    
    if (!wasmSdk.validate_address(mainnetAddress, "mainnet")) {
        throw new Error('Failed to validate mainnet address');
    }
    
    if (!wasmSdk.validate_address(testnetAddress, "testnet")) {
        throw new Error('Failed to validate testnet address');
    }
    
    if (wasmSdk.validate_address(mainnetAddress, "testnet")) {
        throw new Error('Mainnet address should not be valid on testnet');
    }
});

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);
process.exit(failed > 0 ? 1 : 0);