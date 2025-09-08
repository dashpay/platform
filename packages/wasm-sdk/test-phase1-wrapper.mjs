#!/usr/bin/env node

/**
 * Phase 1 Key Generation Functions Test
 * Tests the newly implemented wrapper methods against direct WASM calls
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up Node.js environment for WASM
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Import both WASM SDK and JavaScript wrapper
import init, * as wasmSdk from './pkg/wasm_sdk.js';
import { WasmSDK } from './src-js/index.js';

console.log('🧪 Phase 1 Key Generation Functions Test');
console.log('='.repeat(50));

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
        console.log(`   Error: ${error.message}`);
        failed++;
    }
}

async function compareResults(wrapperResult, wasmResult, testName) {
    const wrapperJson = JSON.stringify(wrapperResult, null, 2);
    const wasmJson = JSON.stringify(wasmResult, null, 2);
    
    if (wrapperJson === wasmJson) {
        console.log(`   ✓ Results match for ${testName}`);
        return true;
    } else {
        console.log(`   ✗ Results differ for ${testName}`);
        console.log(`   Wrapper: ${wrapperJson.substring(0, 100)}...`);
        console.log(`   WASM: ${wasmJson.substring(0, 100)}...`);
        return false;
    }
}

async function main() {
    try {
        // Initialize WASM
        console.log('📦 Initializing WASM...');
        const wasmPath = join(__dirname, 'pkg/wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        // Initialize JavaScript wrapper
        console.log('📦 Initializing JavaScript wrapper...');
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: false
        });
        await sdk.initialize();
        
        console.log('✅ Both SDKs initialized successfully\n');
        
        // Test 1: Generate Mnemonic
        await test('generateMnemonic(12) - wrapper vs WASM', async () => {
            const wrapperResult = await sdk.generateMnemonic(12);
            const wasmResult = wasmSdk.generate_mnemonic(12);
            
            // Both should generate valid 12-word mnemonics
            const wrapperWords = wrapperResult.split(' ');
            const wasmWords = wasmResult.split(' ');
            
            if (wrapperWords.length !== 12) {
                throw new Error(`Wrapper generated ${wrapperWords.length} words, expected 12`);
            }
            if (wasmWords.length !== 12) {
                throw new Error(`WASM generated ${wasmWords.length} words, expected 12`);
            }
            console.log(`   ✓ Both generated 12-word mnemonics`);
        });
        
        // Test 2: Validate Mnemonic
        await test('validateMnemonic() - wrapper vs WASM', async () => {
            const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
            
            const wrapperResult = await sdk.validateMnemonic(testMnemonic);
            const wasmResult = wasmSdk.validate_mnemonic(testMnemonic);
            
            if (wrapperResult !== wasmResult) {
                throw new Error(`Results differ: wrapper=${wrapperResult}, wasm=${wasmResult}`);
            }
            if (!wrapperResult) {
                throw new Error(`Both returned false for valid test mnemonic`);
            }
            console.log(`   ✓ Both returned ${wrapperResult} for valid mnemonic`);
        });
        
        // Test 3: Mnemonic to Seed
        await test('mnemonicToSeed() - wrapper vs WASM', async () => {
            const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
            
            const wrapperResult = await sdk.mnemonicToSeed(testMnemonic);
            const wasmResult = wasmSdk.mnemonic_to_seed(testMnemonic, '');
            
            // Convert to comparable format
            const wrapperArray = Array.from(wrapperResult);
            const wasmArray = Array.from(wasmResult);
            
            if (JSON.stringify(wrapperArray) !== JSON.stringify(wasmArray)) {
                throw new Error(`Seed arrays differ`);
            }
            console.log(`   ✓ Both generated identical ${wrapperArray.length}-byte seeds`);
        });
        
        // Test 4: Derive Key From Seed With Path
        await test('deriveKeyFromSeedWithPath() - wrapper vs WASM', async () => {
            const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
            const testPath = "m/44'/5'/0'/0/0";
            
            const wrapperResult = await sdk.deriveKeyFromSeedWithPath(testMnemonic, '', testPath, 'testnet');
            const wasmResult = wasmSdk.derive_key_from_seed_with_path(testMnemonic, '', testPath, 'testnet');
            
            if (!compareResults(wrapperResult, wasmResult, 'deriveKeyFromSeedWithPath')) {
                throw new Error(`Key derivation results differ`);
            }
            
            // Verify required fields exist
            if (!wrapperResult.address || !wrapperResult.private_key_wif || !wrapperResult.public_key) {
                throw new Error(`Missing required fields in wrapper result`);
            }
            console.log(`   ✓ Generated address: ${wrapperResult.address}`);
        });
        
        // Test 5: Generate Key Pair
        await test('generateKeyPair() - wrapper vs WASM', async () => {
            const wrapperResult = await sdk.generateKeyPair('testnet');
            const wasmResult = wasmSdk.generate_key_pair('testnet');
            
            console.log(`   Debug: wrapper result keys:`, Object.keys(wrapperResult || {}));
            console.log(`   Debug: wasm result keys:`, Object.keys(wasmResult || {}));
            console.log(`   Debug: wrapper result:`, JSON.stringify(wrapperResult, null, 2).substring(0, 200));
            console.log(`   Debug: wasm result:`, JSON.stringify(wasmResult, null, 2).substring(0, 200));
            
            // Check what fields exist in both results
            const wrapperKeys = Object.keys(wrapperResult || {});
            const wasmKeys = Object.keys(wasmResult || {});
            
            if (wrapperKeys.length === 0) {
                throw new Error(`Wrapper result is empty or undefined`);
            }
            if (wasmKeys.length === 0) {
                throw new Error(`WASM result is empty or undefined`);
            }
            
            // Look for key fields with any reasonable name
            const hasWrapperKeys = wrapperKeys.some(key => key.includes('private') || key.includes('public') || key.includes('key'));
            const hasWasmKeys = wasmKeys.some(key => key.includes('private') || key.includes('public') || key.includes('key'));
            
            if (!hasWrapperKeys) {
                throw new Error(`Missing key-related fields in wrapper result. Found: ${wrapperKeys.join(', ')}`);
            }
            if (!hasWasmKeys) {
                throw new Error(`Missing key-related fields in WASM result. Found: ${wasmKeys.join(', ')}`);
            }
            
            console.log(`   ✓ Both generated key pairs with required fields`);
        });
        
        // Test 6: Public Key to Address
        await test('pubkeyToAddress() - wrapper vs WASM', async () => {
            // Generate a key pair first to get a valid public key
            const keyPair = wasmSdk.generate_key_pair('testnet');
            const publicKey = keyPair.public_key;
            
            const wrapperResult = await sdk.pubkeyToAddress(publicKey, 'testnet');
            const wasmResult = wasmSdk.pubkey_to_address(publicKey, 'testnet');
            
            if (wrapperResult !== wasmResult) {
                throw new Error(`Address results differ: wrapper=${wrapperResult}, wasm=${wasmResult}`);
            }
            console.log(`   ✓ Both generated address: ${wrapperResult}`);
        });
        
        // Test 7: Validate Address
        await test('validateAddress() - wrapper vs WASM', async () => {
            // Generate a real address to test with
            const keyPair = wasmSdk.generate_key_pair('testnet');
            const testAddress = wasmSdk.pubkey_to_address(keyPair.public_key, 'testnet');
            const invalidAddress = "invalid_address";
            
            // Test valid address
            const wrapperValid = await sdk.validateAddress(testAddress, 'testnet');
            const wasmValid = wasmSdk.validate_address(testAddress, 'testnet');
            
            if (wrapperValid !== wasmValid) {
                throw new Error(`Valid address results differ: wrapper=${wrapperValid}, wasm=${wasmValid}`);
            }
            
            // Test invalid address
            const wrapperInvalid = await sdk.validateAddress(invalidAddress, 'testnet');
            const wasmInvalid = wasmSdk.validate_address(invalidAddress, 'testnet');
            
            if (wrapperInvalid !== wasmInvalid) {
                throw new Error(`Invalid address results differ: wrapper=${wrapperInvalid}, wasm=${wasmInvalid}`);
            }
            
            console.log(`   ✓ Valid address: ${wrapperValid}, Invalid address: ${wrapperInvalid}`);
        });
        
        // Test 8: Sign Message
        await test('signMessage() - wrapper vs WASM', async () => {
            const testMessage = "Hello, Dash Platform!";
            // Generate a key pair to get a valid private key
            const keyPair = wasmSdk.generate_key_pair('testnet');
            console.log(`   Debug: keyPair keys:`, Object.keys(keyPair || {}));
            console.log(`   Debug: keyPair:`, JSON.stringify(keyPair, null, 2).substring(0, 300));
            
            // Find the private key field
            const privateKeyField = Object.keys(keyPair).find(key => key.includes('private'));
            const privateKey = keyPair[privateKeyField];
            
            console.log(`   Debug: Using private key field '${privateKeyField}':`, privateKey ? privateKey.substring(0, 20) + '...' : 'undefined');
            
            if (!privateKey) {
                throw new Error(`No private key found in keyPair. Available fields: ${Object.keys(keyPair).join(', ')}`);
            }
            
            const wrapperResult = await sdk.signMessage(testMessage, privateKey);
            const wasmResult = wasmSdk.sign_message(testMessage, privateKey);
            
            if (wrapperResult !== wasmResult) {
                throw new Error(`Signature results differ`);
            }
            console.log(`   ✓ Both generated identical signature: ${wrapperResult.substring(0, 20)}...`);
        });
        
        // Clean up
        await sdk.destroy();
        
        console.log(`\n🎉 Phase 1 Test Results:`);
        console.log(`✅ Passed: ${passed}`);
        console.log(`❌ Failed: ${failed}`);
        console.log(`📊 Total: ${passed + failed}`);
        
        if (failed === 0) {
            console.log(`\n🚀 Phase 1 COMPLETE! All key generation functions working correctly.`);
            console.log(`Ready to migrate key generation tests to use JavaScript wrapper.`);
        } else {
            console.log(`\n⚠️ Phase 1 has ${failed} failing tests. Fix before proceeding.`);
        }
        
    } catch (error) {
        console.log(`❌ Test setup failed: ${error.message}`);
        process.exit(1);
    }
}

await main();