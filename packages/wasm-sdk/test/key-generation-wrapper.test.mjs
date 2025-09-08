#!/usr/bin/env node
// key-generation-wrapper.test.mjs - Tests for wallet and key generation functions using JavaScript wrapper

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

// Import JavaScript wrapper (the correct approach)
import { WasmSDK } from '../src-js/index.js';

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

console.log('\nKey Generation Tests Using JavaScript Wrapper (Phase 1 Migration)\n');

// Initialize JavaScript wrapper (modern pattern)
console.log('📦 Initializing JavaScript wrapper...');
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: false,
    debug: false
});
await sdk.initialize();
console.log('✅ JavaScript wrapper initialized successfully\n');

// Key Generation Tests
describe('Mnemonic Generation Tests');

await test('generate 12-word mnemonic', async () => {
    const mnemonic = await sdk.generateMnemonic(12);
    const words = mnemonic.split(' ');
    if (words.length !== 12) {
        throw new Error(`Expected 12 words, got ${words.length}`);
    }
    console.log(`   Generated: ${mnemonic.substring(0, 40)}...`);
});

await test('generate 24-word mnemonic', async () => {
    const mnemonic = await sdk.generateMnemonic(24);
    const words = mnemonic.split(' ');
    if (words.length !== 24) {
        throw new Error(`Expected 24 words, got ${words.length}`);
    }
    console.log(`   Generated: ${mnemonic.substring(0, 40)}...`);
});

await test('invalid word count should throw error', async () => {
    try {
        await sdk.generateMnemonic(13); // Invalid count
        throw new Error('Should have thrown an error for invalid word count');
    } catch (error) {
        if (!error.message.includes('Invalid word count')) {
            throw new Error(`Expected word count error, got: ${error.message}`);
        }
        console.log('   ✓ Correctly rejected invalid word count');
    }
});

describe('Mnemonic Validation Tests');

await test('validate valid mnemonic', async () => {
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const isValid = await sdk.validateMnemonic(testMnemonic);
    if (!isValid) {
        throw new Error('Known valid mnemonic was rejected');
    }
});

await test('reject invalid mnemonic', async () => {
    const invalidMnemonic = "invalid invalid invalid invalid invalid invalid invalid invalid invalid invalid invalid invalid";
    const isValid = await sdk.validateMnemonic(invalidMnemonic);
    if (isValid) {
        throw new Error('Invalid mnemonic was accepted');
    }
});

await test('generated mnemonics should validate', async () => {
    const mnemonic = await sdk.generateMnemonic(12);
    const isValid = await sdk.validateMnemonic(mnemonic);
    if (!isValid) {
        throw new Error('Generated mnemonic failed validation');
    }
});

describe('Seed Derivation Tests');

await test('mnemonic to seed conversion', async () => {
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const seed = await sdk.mnemonicToSeed(testMnemonic);
    
    if (!(seed instanceof Uint8Array)) {
        throw new Error(`Expected Uint8Array, got ${typeof seed}`);
    }
    if (seed.length !== 64) {
        throw new Error(`Expected 64 bytes, got ${seed.length}`);
    }
    console.log(`   Generated ${seed.length}-byte seed`);
});

await test('mnemonic to seed with passphrase', async () => {
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const seedNoPass = await sdk.mnemonicToSeed(testMnemonic);
    const seedWithPass = await sdk.mnemonicToSeed(testMnemonic, "test passphrase");
    
    if (Array.from(seedNoPass).join(',') === Array.from(seedWithPass).join(',')) {
        throw new Error('Seeds should be different with different passphrases');
    }
    console.log('   ✓ Different passphrases produce different seeds');
});

describe('Key Derivation Tests');

await test('derive key from seed with path', async () => {
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const path = "m/44'/5'/0'/0/0";
    
    const result = await sdk.deriveKeyFromSeedWithPath(testMnemonic, '', path, 'testnet');
    
    if (!result.address || !result.private_key_wif || !result.public_key) {
        throw new Error('Missing required fields in result');
    }
    
    console.log(`   Address: ${result.address}`);
    console.log(`   Public key: ${result.public_key.substring(0, 20)}...`);
});

await test('consistent derivation', async () => {
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const path = "m/44'/5'/0'/0/0";
    
    const result1 = await sdk.deriveKeyFromSeedWithPath(testMnemonic, '', path, 'testnet');
    const result2 = await sdk.deriveKeyFromSeedWithPath(testMnemonic, '', path, 'testnet');
    
    if (result1.address !== result2.address) {
        throw new Error('Same inputs should produce same results');
    }
    console.log('   ✓ Consistent derivation confirmed');
});

await test('different networks produce different addresses', async () => {
    const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const path = "m/44'/5'/0'/0/0";
    
    const testnetResult = await sdk.deriveKeyFromSeedWithPath(testMnemonic, '', path, 'testnet');
    const mainnetResult = await sdk.deriveKeyFromSeedWithPath(testMnemonic, '', path, 'mainnet');
    
    if (testnetResult.address === mainnetResult.address) {
        throw new Error('Different networks should produce different addresses');
    }
    console.log(`   Testnet: ${testnetResult.address}`);
    console.log(`   Mainnet: ${mainnetResult.address}`);
});

describe('Key Pair Generation Tests');

await test('generate random key pair', async () => {
    const keyPair = await sdk.generateKeyPair('testnet');
    
    if (!keyPair.private_key_wif || !keyPair.public_key || !keyPair.address) {
        throw new Error('Missing required fields in key pair');
    }
    
    console.log(`   Address: ${keyPair.address}`);
    console.log(`   Public key: ${keyPair.public_key.substring(0, 20)}...`);
});

await test('generated key pairs should be random', async () => {
    const keyPair1 = await sdk.generateKeyPair('testnet');
    const keyPair2 = await sdk.generateKeyPair('testnet');
    
    if (keyPair1.address === keyPair2.address) {
        throw new Error('Generated key pairs should be different');
    }
    console.log('   ✓ Key pairs are properly randomized');
});

describe('Address Validation Tests');

await test('validate generated addresses', async () => {
    const keyPair = await sdk.generateKeyPair('testnet');
    const isValid = await sdk.validateAddress(keyPair.address, 'testnet');
    
    if (!isValid) {
        throw new Error('Generated address failed validation');
    }
});

await test('reject invalid addresses', async () => {
    const invalidAddress = "invalid_address_123";
    const isValid = await sdk.validateAddress(invalidAddress, 'testnet');
    
    if (isValid) {
        throw new Error('Invalid address was accepted');
    }
});

await test('network-specific validation', async () => {
    const keyPair = await sdk.generateKeyPair('testnet');
    
    const testnetValid = await sdk.validateAddress(keyPair.address, 'testnet');
    const mainnetValid = await sdk.validateAddress(keyPair.address, 'mainnet');
    
    if (!testnetValid) {
        throw new Error('Testnet address should be valid on testnet');
    }
    if (mainnetValid) {
        throw new Error('Testnet address should not be valid on mainnet');
    }
    console.log('   ✓ Network-specific validation working');
});

describe('Public Key to Address Tests');

await test('convert public key to address', async () => {
    const keyPair = await sdk.generateKeyPair('testnet');
    const derivedAddress = await sdk.pubkeyToAddress(keyPair.public_key, 'testnet');
    
    if (derivedAddress !== keyPair.address) {
        throw new Error('Derived address does not match key pair address');
    }
    console.log('   ✓ Public key correctly converts to address');
});

describe('Message Signing Tests');

await test('sign and verify message consistency', async () => {
    const keyPair1 = await sdk.generateKeyPair('testnet');
    const keyPair2 = await sdk.generateKeyPair('testnet');
    
    const message = "Hello, Dash Platform!";
    
    const signature1a = await sdk.signMessage(message, keyPair1.private_key_wif);
    const signature1b = await sdk.signMessage(message, keyPair1.private_key_wif);
    const signature2 = await sdk.signMessage(message, keyPair2.private_key_wif);
    
    if (signature1a !== signature1b) {
        throw new Error('Same key should produce same signature');
    }
    if (signature1a === signature2) {
        throw new Error('Different keys should produce different signatures');
    }
    
    console.log('   ✓ Message signing is consistent and unique');
});

// Clean up resources
await sdk.destroy();

console.log(`\n\n🎉 Key Generation Wrapper Test Results:`);
console.log(`✅ Passed: ${passed}`);
console.log(`❌ Failed: ${failed}`);
console.log(`📊 Total: ${passed + failed}`);

if (failed === 0) {
    console.log(`\n🚀 SUCCESS! All key generation functions work correctly with JavaScript wrapper.`);
    console.log(`This test file demonstrates the correct pattern for using the wrapper instead of direct WASM.`);
} else {
    console.log(`\n⚠️ ${failed} tests failed. Need to investigate wrapper implementation.`);
}

process.exit(failed > 0 ? 1 : 0);