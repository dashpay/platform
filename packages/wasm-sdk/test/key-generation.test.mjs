#!/usr/bin/env node
// key-generation.test.mjs - Tests for wallet and key generation functions

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
import { WasmSDK } from '../src-js/index.js';

// Pre-load WASM for Node.js compatibility
console.log('Initializing WASM module...');
const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

// Initialize JavaScript wrapper
console.log('Initializing JavaScript wrapper...');
const sdk = new WasmSDK({
    network: 'testnet',
    proofs: false,
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

// Test constants
const TEST_MNEMONIC = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

console.log('\nKey Generation Tests\n');

// Mnemonic Generation Tests
describe('Mnemonic Generation');

await test('generateMnemonic - 12 words (default)', async () => {
    const mnemonic = await sdk.generateMnemonic();
    const words = mnemonic.split(' ');
    if (words.length !== 12) {
        throw new Error(`Expected 12 words, got ${words.length}`);
    }
    if (!(await sdk.validateMnemonic(mnemonic))) {
        throw new Error('Generated mnemonic is invalid');
    }
});

await test('generateMnemonic - 15 words', async () => {
    const mnemonic = await sdk.generateMnemonic(15);
    const words = mnemonic.split(' ');
    if (words.length !== 15) {
        throw new Error(`Expected 15 words, got ${words.length}`);
    }
    if (!(await sdk.validateMnemonic(mnemonic))) {
        throw new Error('Generated mnemonic is invalid');
    }
});

await test('generateMnemonic - 18 words', async () => {
    const mnemonic = await sdk.generateMnemonic(18);
    const words = mnemonic.split(' ');
    if (words.length !== 18) {
        throw new Error(`Expected 18 words, got ${words.length}`);
    }
    if (!(await sdk.validateMnemonic(mnemonic))) {
        throw new Error('Generated mnemonic is invalid');
    }
});

await test('generateMnemonic - 21 words', async () => {
    const mnemonic = await sdk.generateMnemonic(21);
    const words = mnemonic.split(' ');
    if (words.length !== 21) {
        throw new Error(`Expected 21 words, got ${words.length}`);
    }
    if (!(await sdk.validateMnemonic(mnemonic))) {
        throw new Error('Generated mnemonic is invalid');
    }
});

await test('generateMnemonic - 24 words', async () => {
    const mnemonic = await sdk.generateMnemonic(24);
    const words = mnemonic.split(' ');
    if (words.length !== 24) {
        throw new Error(`Expected 24 words, got ${words.length}`);
    }
    if (!(await sdk.validateMnemonic(mnemonic))) {
        throw new Error('Generated mnemonic is invalid');
    }
});

await test('generate_mnemonic - invalid word count', () => {
    try {
        await sdk.generateMnemonic(13);
        throw new Error('Should have thrown error for invalid word count');
    } catch (error) {
        if (!error.message.includes('Word count must be')) {
            throw error;
        }
    }
});

// Language-specific mnemonic tests
describe('Mnemonic Languages');

const languages = [
    { code: 'en', name: 'English' },
    { code: 'es', name: 'Spanish' },
    { code: 'fr', name: 'French' },
    { code: 'it', name: 'Italian' },
    { code: 'ja', name: 'Japanese' },
    { code: 'ko', name: 'Korean' },
    { code: 'pt', name: 'Portuguese' },
    { code: 'cs', name: 'Czech' },
    { code: 'zh-cn', name: 'Simplified Chinese' },
    { code: 'zh-tw', name: 'Traditional Chinese' }
];

for (const { code, name } of languages) {
    await test(`generate_mnemonic - ${name} (${code})`, () => {
        const mnemonic = await sdk.generateMnemonic(12, code);
        if (!await sdk.validateMnemonic(mnemonic, code)) {
            throw new Error(`Generated ${name} mnemonic is invalid`);
        }
    });
}

await test('generate_mnemonic - unsupported language', () => {
    try {
        await sdk.generateMnemonic(12, 'xx');
        throw new Error('Should have thrown error for unsupported language');
    } catch (error) {
        if (!error.message.includes('Unsupported language code')) {
            throw error;
        }
    }
});

// Mnemonic Validation Tests
describe('Mnemonic Validation');

await test('validateMnemonic - valid mnemonic', async () => {
    if (!(await sdk.validateMnemonic(TEST_MNEMONIC))) {
        throw new Error('Test mnemonic should be valid');
    }
});

await test('validate_mnemonic - invalid checksum', () => {
    const invalidMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
    if (await sdk.validateMnemonic(invalidMnemonic)) {
        throw new Error('Invalid mnemonic should not validate');
    }
});

await test('validate_mnemonic - wrong word count', () => {
    const invalidMnemonic = "abandon abandon abandon";
    if (await sdk.validateMnemonic(invalidMnemonic)) {
        throw new Error('Mnemonic with wrong word count should not validate');
    }
});

// Mnemonic to Seed Tests
describe('Mnemonic to Seed');

await test('mnemonic_to_seed - without passphrase', () => {
    const seed = await sdk.mnemonicToSeed(TEST_MNEMONIC);
    if (!seed || seed.length !== 64) {
        throw new Error(`Expected 64 byte seed, got ${seed ? seed.length : 'null'}`);
    }
});

await test('mnemonic_to_seed - with passphrase', () => {
    const seed1 = await sdk.mnemonicToSeed(TEST_MNEMONIC, "passphrase");
    const seed2 = await sdk.mnemonicToSeed(TEST_MNEMONIC);
    
    if (!seed1 || seed1.length !== 64) {
        throw new Error('Seed with passphrase should be 64 bytes');
    }
    
    // Seeds should be different with different passphrases
    if (seed1.toString() === seed2.toString()) {
        throw new Error('Seeds should differ with different passphrases');
    }
});

await test('mnemonic_to_seed - invalid mnemonic', () => {
    try {
        await sdk.mnemonicToSeed("invalid mnemonic phrase");
        throw new Error('Should have thrown error for invalid mnemonic');
    } catch (error) {
        if (!error.message.includes('Invalid mnemonic')) {
            throw error;
        }
    }
});

// Key Derivation Tests
describe('Key Derivation from Seed');

await test('derive_key_from_seed_phrase - mainnet', () => {
    const result = wasmSdk.derive_key_from_seed_phrase(TEST_MNEMONIC, null, "mainnet");
    
    if (!result.private_key_wif) throw new Error('Missing private_key_wif');
    if (!result.private_key_hex) throw new Error('Missing private_key_hex');
    if (!result.public_key) throw new Error('Missing public_key');
    if (!result.address) throw new Error('Missing address');
    if (result.network !== "mainnet") throw new Error('Wrong network');
    
    // Mainnet addresses should start with 'X'
    if (!result.address.startsWith('X')) {
        throw new Error(`Mainnet address should start with 'X', got ${result.address}`);
    }
});

await test('derive_key_from_seed_phrase - testnet', () => {
    const result = wasmSdk.derive_key_from_seed_phrase(TEST_MNEMONIC, null, "testnet");
    
    if (!result.private_key_wif) throw new Error('Missing private_key_wif');
    if (!result.address) throw new Error('Missing address');
    if (result.network !== "testnet") throw new Error('Wrong network');
    
    // Testnet addresses should start with 'y'
    if (!result.address.startsWith('y')) {
        throw new Error(`Testnet address should start with 'y', got ${result.address}`);
    }
});

// Derivation Path Tests
describe('Derivation Path Functions');

await test('derive_key_from_seed_with_path - BIP44 mainnet', () => {
    const path = "m/44'/5'/0'/0/0";
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, null, path, "mainnet");
    
    if (!result.path || result.path !== path) throw new Error('Path mismatch');
    if (!result.private_key_wif) throw new Error('Missing private_key_wif');
    if (!result.address) throw new Error('Missing address');
    if (!result.address.startsWith('X')) throw new Error('Invalid mainnet address');
});

await test('derive_key_from_seed_with_path - BIP44 testnet', () => {
    const path = "m/44'/1'/0'/0/0";
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, null, path, "testnet");
    
    if (!result.path || result.path !== path) throw new Error('Path mismatch');
    if (!result.address) throw new Error('Missing address');
    if (!result.address.startsWith('y')) throw new Error('Invalid testnet address');
});

await test('derive_key_from_seed_with_path - DIP13 path', () => {
    const path = "m/9'/5'/5'/0'/0'/0'/0'";
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, null, path, "mainnet");
    
    if (!result.path || result.path !== path) throw new Error('Path mismatch');
    if (!result.private_key_wif) throw new Error('Missing private_key_wif');
    if (!result.address) throw new Error('Missing address');
});

await test('derive_key_from_seed_with_path - invalid path', () => {
    try {
        await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, null, "invalid/path", "mainnet");
        throw new Error('Should have thrown error for invalid path');
    } catch (error) {
        if (!error.message.includes('Invalid derivation path')) {
            throw error;
        }
    }
});

// Path Generation Tests
describe('Derivation Path Helpers');

await test('derivation_path_bip44_mainnet', () => {
    const result = wasmSdk.derivation_path_bip44_mainnet(0, 0, 0);
    if (!result) throw new Error('Missing result');
    if (result.purpose !== 44) throw new Error('Wrong purpose');
    if (result.coin_type !== 5) throw new Error('Wrong coin type');
    if (result.account !== 0) throw new Error('Wrong account');
    if (result.change !== 0) throw new Error('Wrong change');
    if (result.index !== 0) throw new Error('Wrong index');
    // Build expected path
    const expectedPath = `m/${result.purpose}'/${result.coin_type}'/${result.account}'/${result.change}/${result.index}`;
    if (expectedPath !== "m/44'/5'/0'/0/0") throw new Error('Invalid BIP44 mainnet path components');
});

await test('derivation_path_bip44_testnet', () => {
    const result = wasmSdk.derivation_path_bip44_testnet(0, 0, 0);
    if (!result) throw new Error('Missing result');
    if (result.purpose !== 44) throw new Error('Wrong purpose');
    if (result.coin_type !== 1) throw new Error('Wrong coin type');
    if (result.account !== 0) throw new Error('Wrong account');
    if (result.change !== 0) throw new Error('Wrong change');
    if (result.index !== 0) throw new Error('Wrong index');
    // Build expected path
    const expectedPath = `m/${result.purpose}'/${result.coin_type}'/${result.account}'/${result.change}/${result.index}`;
    if (expectedPath !== "m/44'/1'/0'/0/0") throw new Error('Invalid BIP44 testnet path components');
});

await test('derivation_path_dip9_mainnet', () => {
    const result = wasmSdk.derivation_path_dip9_mainnet(5, 0, 0);
    if (!result) throw new Error('Missing result');
    if (result.purpose !== 9) throw new Error('Wrong purpose');
    if (result.coin_type !== 5) throw new Error('Wrong coin type');
    if (result.account !== 5) throw new Error('Wrong account');
    if (result.change !== 0) throw new Error('Wrong change');
    if (result.index !== 0) throw new Error('Wrong index');
    // Build expected path
    const expectedPath = `m/${result.purpose}'/${result.coin_type}'/${result.account}'/${result.change}/${result.index}`;
    if (expectedPath !== "m/9'/5'/5'/0/0") throw new Error('Invalid DIP9 mainnet path components');
});

await test('derivation_path_dip9_testnet', () => {
    const result = wasmSdk.derivation_path_dip9_testnet(5, 0, 0);
    if (!result) throw new Error('Missing result');
    if (result.purpose !== 9) throw new Error('Wrong purpose');
    if (result.coin_type !== 1) throw new Error('Wrong coin type');
    if (result.account !== 5) throw new Error('Wrong account');
    if (result.change !== 0) throw new Error('Wrong change');
    if (result.index !== 0) throw new Error('Wrong index');
    // Build expected path
    const expectedPath = `m/${result.purpose}'/${result.coin_type}'/${result.account}'/${result.change}/${result.index}`;
    if (expectedPath !== "m/9'/1'/5'/0/0") throw new Error('Invalid DIP9 testnet path components');
});

await test('derivation_path_dip13_mainnet', () => {
    const result = wasmSdk.derivation_path_dip13_mainnet(0);
    if (!result || !result.path) throw new Error('Missing path');
    if (result.path !== "m/9'/5'/0'") throw new Error('Invalid DIP13 mainnet path');
    if (result.purpose !== 9) throw new Error('Wrong purpose');
    if (result.description !== "DIP13 HD identity key path") throw new Error('Wrong description');
});

await test('derivation_path_dip13_testnet', () => {
    const result = wasmSdk.derivation_path_dip13_testnet(0);
    if (!result || !result.path) throw new Error('Missing path');
    if (result.path !== "m/9'/1'/0'") throw new Error('Invalid DIP13 testnet path');
});

// Child Key Derivation Tests (expected to fail for now)
describe('Child Key Derivation');

await test('derive_child_public_key - not implemented', () => {
    try {
        wasmSdk.derive_child_public_key("xpub...", 0, false);
        throw new Error('Should have thrown not implemented error');
    } catch (error) {
        if (!error.message.includes('not yet implemented')) {
            throw error;
        }
    }
});

await test('xprv_to_xpub - not implemented', () => {
    try {
        wasmSdk.xprv_to_xpub("xprv...");
        throw new Error('Should have thrown not implemented error');
    } catch (error) {
        if (!error.message.includes('not yet implemented')) {
            throw error;
        }
    }
});

// Key Pair Generation Tests
describe('Key Pair Generation');

await test('generate_key_pair - mainnet', () => {
    const keyPair = await sdk.generateKeyPair("mainnet");
    
    if (!keyPair.private_key_wif) throw new Error('Missing private_key_wif');
    if (!keyPair.private_key_hex) throw new Error('Missing private_key_hex');
    if (!keyPair.public_key) throw new Error('Missing public_key');
    if (!keyPair.address) throw new Error('Missing address');
    if (keyPair.network !== "mainnet") throw new Error('Wrong network');
    if (!keyPair.address.startsWith('X')) throw new Error('Invalid mainnet address');
});

await test('generate_key_pair - testnet', () => {
    const keyPair = await sdk.generateKeyPair("testnet");
    
    if (!keyPair.address) throw new Error('Missing address');
    if (keyPair.network !== "testnet") throw new Error('Wrong network');
    if (!keyPair.address.startsWith('y')) throw new Error('Invalid testnet address');
});

await test('generate_key_pairs - multiple', () => {
    const keyPairs = wasmSdk.generate_key_pairs("mainnet", 3);
    
    if (!Array.isArray(keyPairs)) throw new Error('Should return array');
    if (keyPairs.length !== 3) throw new Error(`Expected 3 key pairs, got ${keyPairs.length}`);
    
    // Check each key pair
    for (let i = 0; i < keyPairs.length; i++) {
        const kp = keyPairs[i];
        if (!kp.address) throw new Error(`Key pair ${i} missing address`);
        if (!kp.address.startsWith('X')) throw new Error(`Key pair ${i} invalid address`);
    }
    
    // Ensure all addresses are unique
    const addresses = keyPairs.map(kp => kp.address);
    const uniqueAddresses = new Set(addresses);
    if (uniqueAddresses.size !== addresses.length) {
        throw new Error('Generated duplicate addresses');
    }
});

// Key Import Tests
describe('Key Import');

await test('key_pair_from_wif - mainnet', () => {
    // First generate a key pair to get a valid WIF
    const generated = await sdk.generateKeyPair("mainnet");
    const imported = wasmSdk.key_pair_from_wif(generated.private_key_wif);
    
    if (imported.address !== generated.address) {
        throw new Error('Imported address does not match');
    }
    if (imported.public_key !== generated.public_key) {
        throw new Error('Imported public key does not match');
    }
});

await test('key_pair_from_wif - invalid WIF', () => {
    try {
        wasmSdk.key_pair_from_wif("invalid_wif");
        throw new Error('Should have thrown error for invalid WIF');
    } catch (error) {
        // Expected error
    }
});

await test('key_pair_from_hex - mainnet', () => {
    // Generate a key pair to get valid hex
    const generated = await sdk.generateKeyPair("mainnet");
    const imported = wasmSdk.key_pair_from_hex(generated.private_key_hex, "mainnet");
    
    if (imported.address !== generated.address) {
        throw new Error('Imported address does not match');
    }
    if (imported.public_key !== generated.public_key) {
        throw new Error('Imported public key does not match');
    }
});

await test('key_pair_from_hex - invalid hex', () => {
    try {
        wasmSdk.key_pair_from_hex("invalid_hex", "mainnet");
        throw new Error('Should have thrown error for invalid hex');
    } catch (error) {
        // Expected error
    }
});

// Address Operations Tests
describe('Address Operations');

await test('pubkey_to_address - mainnet', () => {
    const keyPair = await sdk.generateKeyPair("mainnet");
    const address = await sdk.pubkeyToAddress(keyPair.public_key, "mainnet");
    
    if (address !== keyPair.address) {
        throw new Error('Address from public key does not match');
    }
});

await test('pubkey_to_address - testnet', () => {
    const keyPair = await sdk.generateKeyPair("testnet");
    const address = await sdk.pubkeyToAddress(keyPair.public_key, "testnet");
    
    if (address !== keyPair.address) {
        throw new Error('Address from public key does not match');
    }
});

await test('validate_address - valid mainnet', () => {
    const keyPair = await sdk.generateKeyPair("mainnet");
    if (!await sdk.validateAddress(keyPair.address, "mainnet")) {
        throw new Error('Valid mainnet address should validate');
    }
});

await test('validate_address - valid testnet', () => {
    const keyPair = await sdk.generateKeyPair("testnet");
    if (!await sdk.validateAddress(keyPair.address, "testnet")) {
        throw new Error('Valid testnet address should validate');
    }
});

await test('validate_address - wrong network', () => {
    const mainnetKey = await sdk.generateKeyPair("mainnet");
    const testnetKey = await sdk.generateKeyPair("testnet");
    
    if (await sdk.validateAddress(mainnetKey.address, "testnet")) {
        throw new Error('Mainnet address should not validate on testnet');
    }
    if (await sdk.validateAddress(testnetKey.address, "mainnet")) {
        throw new Error('Testnet address should not validate on mainnet');
    }
});

await test('validate_address - invalid address', () => {
    if (await sdk.validateAddress("invalid_address", "mainnet")) {
        throw new Error('Invalid address should not validate');
    }
});

// Message Signing Tests
describe('Message Signing');

await test('sign_message - basic', () => {
    const keyPair = await sdk.generateKeyPair("mainnet");
    const message = "Hello, Dash!";
    const signature = await sdk.signMessage(message, keyPair.private_key_wif);
    
    if (!signature) throw new Error('No signature returned');
    if (typeof signature !== 'string') throw new Error('Signature should be string');
    if (signature.length < 80) throw new Error('Signature too short');
});

await test('sign_message - different messages produce different signatures', () => {
    const keyPair = await sdk.generateKeyPair("mainnet");
    const sig1 = await sdk.signMessage("Message 1", keyPair.private_key_wif);
    const sig2 = await sdk.signMessage("Message 2", keyPair.private_key_wif);
    
    if (sig1 === sig2) {
        throw new Error('Different messages should produce different signatures');
    }
});

await test('sign_message - same message produces same signature', () => {
    const keyPair = await sdk.generateKeyPair("mainnet");
    const message = "Test message";
    const sig1 = await sdk.signMessage(message, keyPair.private_key_wif);
    const sig2 = await sdk.signMessage(message, keyPair.private_key_wif);
    
    if (sig1 !== sig2) {
        throw new Error('Same message should produce same signature');
    }
});

// Cleanup
await sdk.destroy();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);
process.exit(failed > 0 ? 1 : 0);