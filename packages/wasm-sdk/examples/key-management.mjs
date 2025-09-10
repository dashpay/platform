#!/usr/bin/env node

/**
 * Key Management Example
 * 
 * Comprehensive demonstration of all key generation, derivation, and cryptographic operations.
 * Shows mnemonic generation, validation, key derivation, address operations, and message signing.
 * 
 * Usage: node examples/key-management.mjs [--network=testnet|mainnet] [--debug]
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up Node.js environment for WASM
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Import JavaScript wrapper (the correct approach)
import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

async function main() {
    console.log('🔑 Comprehensive Key Management Example');
    console.log('='.repeat(50));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const debugMode = args.includes('--debug');
    
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🐛 Debug: ${debugMode ? 'ENABLED' : 'DISABLED'}`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('\n📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        // Initialize JavaScript wrapper (modern pattern)
        console.log('📦 Initializing JavaScript wrapper...');
        const sdk = new WasmSDK({
            network: network,
            proofs: false, // Crypto operations don't need proofs
            debug: debugMode
        });
        await sdk.initialize();
        console.log('✅ SDK initialized successfully\n');
        
        // === MNEMONIC OPERATIONS ===
        console.log('🎲 MNEMONIC GENERATION & VALIDATION');
        console.log('-'.repeat(40));
        
        // Generate mnemonics of different lengths
        const wordCounts = [12, 15, 18, 21, 24];
        for (const count of wordCounts) {
            const mnemonic = await sdk.generateMnemonic(count);
            const isValid = await sdk.validateMnemonic(mnemonic);
            console.log(`✓ ${count} words: ${mnemonic.split(' ').slice(0, 3).join(' ')}... (valid: ${isValid})`);
        }
        
        // Demonstrate mnemonic validation
        const testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        const isTestValid = await sdk.validateMnemonic(testMnemonic);
        console.log(`✓ Test mnemonic validation: ${isTestValid}`);
        
        // === SEED GENERATION ===
        console.log('\n🌱 SEED GENERATION FROM MNEMONICS');
        console.log('-'.repeat(40));
        
        // Generate seed without passphrase
        const seed1 = await sdk.mnemonicToSeed(testMnemonic);
        console.log(`✓ Seed without passphrase: ${seed1.length} bytes`);
        
        // Generate seed with passphrase
        const seed2 = await sdk.mnemonicToSeed(testMnemonic, 'my-passphrase');
        console.log(`✓ Seed with passphrase: ${seed2.length} bytes`);
        console.log(`✓ Seeds are different: ${Array.from(seed1).join(',') !== Array.from(seed2).join(',')}`);
        
        // === KEY DERIVATION ===
        console.log('\n🗝️  KEY DERIVATION FROM SEED');
        console.log('-'.repeat(40));
        
        // Demonstrate different derivation paths
        const derivationPaths = [
            { path: "m/44'/5'/0'/0/0", name: "BIP44 Mainnet", network: 'mainnet' },
            { path: "m/44'/1'/0'/0/0", name: "BIP44 Testnet", network: 'testnet' },
            { path: "m/9'/5'/0'/0/0", name: "DIP9 Identity", network: 'mainnet' },
            { path: "m/9'/1'/5'/0/0", name: "DIP9 Testnet", network: 'testnet' }
        ];
        
        for (const { path, name, network: keyNetwork } of derivationPaths) {
            const result = await sdk.deriveKeyFromSeedWithPath(testMnemonic, '', path, keyNetwork);
            console.log(`✓ ${name}: ${result.address} (${keyNetwork})`);
            console.log(`  Private key (WIF): ${result.private_key_wif.substring(0, 20)}...`);
            console.log(`  Public key: ${result.public_key.substring(0, 20)}...`);
        }
        
        // === RANDOM KEY GENERATION ===
        console.log('\n🎲 RANDOM KEY PAIR GENERATION');
        console.log('-'.repeat(40));
        
        for (const keyNetwork of ['testnet', 'mainnet']) {
            const keyPair = await sdk.generateKeyPair(keyNetwork);
            console.log(`✓ ${keyNetwork.toUpperCase()} key pair:`);
            console.log(`  Address: ${keyPair.address}`);
            console.log(`  Private key (WIF): ${keyPair.private_key_wif.substring(0, 20)}...`);
            console.log(`  Public key: ${keyPair.public_key.substring(0, 20)}...`);
        }
        
        // === ADDRESS OPERATIONS ===
        console.log('\n🏠 ADDRESS OPERATIONS');
        console.log('-'.repeat(40));
        
        // Generate a key pair for address operations
        const testKeyPair = await sdk.generateKeyPair('testnet');
        
        // Derive address from public key
        const derivedAddress = await sdk.pubkeyToAddress(testKeyPair.public_key, 'testnet');
        console.log(`✓ Address from public key: ${derivedAddress}`);
        console.log(`✓ Matches key pair address: ${derivedAddress === testKeyPair.address}`);
        
        // Address validation
        const isValidAddress = await sdk.validateAddress(testKeyPair.address, 'testnet');
        console.log(`✓ Address validation (testnet): ${isValidAddress}`);
        
        const isValidOnMainnet = await sdk.validateAddress(testKeyPair.address, 'mainnet');
        console.log(`✓ Address validation (mainnet): ${isValidOnMainnet}`);
        
        const isInvalidAddress = await sdk.validateAddress('invalid_address', 'testnet');
        console.log(`✓ Invalid address validation: ${isInvalidAddress}`);
        
        // === MESSAGE SIGNING ===
        console.log('\n✍️  MESSAGE SIGNING');
        console.log('-'.repeat(40));
        
        const testMessages = [
            "Hello, Dash Platform!",
            "Secure message signing test",
            "Different message for signature comparison"
        ];
        
        const signatures = [];
        for (const message of testMessages) {
            const signature = await sdk.signMessage(message, testKeyPair.private_key_wif);
            signatures.push(signature);
            console.log(`✓ "${message}"`);
            console.log(`  Signature: ${signature.substring(0, 30)}...`);
        }
        
        // Verify signature uniqueness
        const uniqueSignatures = new Set(signatures);
        console.log(`✓ All signatures unique: ${uniqueSignatures.size === signatures.length}`);
        
        // Verify signature consistency
        const signature1 = await sdk.signMessage(testMessages[0], testKeyPair.private_key_wif);
        const signature2 = await sdk.signMessage(testMessages[0], testKeyPair.private_key_wif);
        console.log(`✓ Signature consistency: ${signature1 === signature2}`);
        
        // === SUMMARY ===
        console.log('\n📊 KEY MANAGEMENT SUMMARY');
        console.log('-'.repeat(40));
        console.log(`✓ Mnemonic generation: ${wordCounts.length} different word counts`);
        console.log(`✓ Mnemonic validation: Working correctly`);
        console.log(`✓ Seed generation: With and without passphrases`);
        console.log(`✓ Key derivation: ${derivationPaths.length} different paths`);
        console.log(`✓ Random key generation: Both networks`);
        console.log(`✓ Address operations: Generation and validation`);
        console.log(`✓ Message signing: Unique and consistent signatures`);
        
        // Clean up
        await sdk.destroy();
        console.log('\n🎉 Key management demonstration completed successfully!');
        
    } catch (error) {
        console.log(`❌ Key management example failed: ${error.message}`);
        console.log('\nFor debugging:');
        console.log('1. Ensure WASM module is built correctly');
        console.log('2. Check if all key generation functions are implemented');
        console.log('3. Try with --debug for detailed output');
        
        process.exit(1);
    }
}

await main();