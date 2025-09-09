#!/usr/bin/env node
// run-tests.mjs - Node.js test runner for WASM SDK with ES modules support

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

// Test utilities
const tests = [];
let currentSuite = '';

function describe(name, fn) {
    currentSuite = name;
    console.log(`\n${name}`);
    fn();
}

function test(name, fn) {
    tests.push({ suite: currentSuite, name, fn });
}

function expect(value) {
    return {
        toBe(expected) {
            if (value !== expected) {
                throw new Error(`Expected ${value} to be ${expected}`);
            }
        },
        toBeDefined() {
            if (value === undefined) {
                throw new Error(`Expected value to be defined`);
            }
        },
        toMatch(pattern) {
            if (!pattern.test(value)) {
                throw new Error(`Expected ${value} to match ${pattern}`);
            }
        }
    };
}

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Define tests
const testSeed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

describe('Key Derivation Tests', () => {
    describe('derive_key_from_seed_with_path', () => {
        test('should derive BIP44 mainnet key', async () => {
            const path = "m/44'/5'/0'/0/0";
            const result = await wasmSdk.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            
            console.log('  BIP44 result:', JSON.stringify(result, null, 2));
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            expect(result.private_key_wif).toBeDefined();
            expect(result.private_key_hex).toBeDefined();
            expect(result.public_key).toBeDefined();
            expect(result.address).toBeDefined();
            expect(result.network).toBe("mainnet");
        });
        
        test('should derive DIP13 authentication key', async () => {
            const path = "m/9'/5'/5'/0'/0'/0'/0'";
            const result = await wasmSdk.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            
            console.log('  DIP13 Authentication key result:', JSON.stringify(result, null, 2));
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            expect(result.private_key_wif).toBeDefined();
            expect(result.address).toBeDefined();
        });
        
        test('should derive DIP13 registration funding key', async () => {
            const path = "m/9'/5'/5'/1'/0";
            const result = await wasmSdk.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            
            console.log('  DIP13 Registration funding key result:', JSON.stringify(result, null, 2));
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            expect(result.private_key_wif).toBeDefined();
        });
        
        test('should work with passphrase', async () => {
            const path = "m/44'/5'/0'/0/0";
            const passphrase = "test passphrase";
            const result = await wasmSdk.derive_key_from_seed_with_path(
                testSeed,
                passphrase,
                path,
                "mainnet"
            );
            
            console.log('  With passphrase result:', JSON.stringify(result, null, 2));
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            
            // Address should be different with passphrase
            const withoutPassphrase = await wasmSdk.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            
            if (result.address === withoutPassphrase.address) {
                throw new Error('Address should be different with passphrase');
            }
        });
        
        test('should work on testnet', async () => {
            const path = "m/44'/1'/0'/0/0";
            const result = await wasmSdk.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "testnet"
            );
            
            console.log('  Testnet result:', JSON.stringify(result, null, 2));
            
            expect(result).toBeDefined();
            expect(result.network).toBe("testnet");
            expect(result.address).toMatch(/^y/); // Testnet addresses start with 'y'
        });
    });
    
    describe('generate_mnemonic', () => {
        test('should generate 12-word mnemonic', async () => {
            const mnemonic = wasmSdk.generate_mnemonic(12);
            const words = mnemonic.split(' ');
            
            console.log('  Generated 12-word mnemonic:', mnemonic);
            
            expect(words.length).toBe(12);
            expect(wasmSdk.validate_mnemonic(mnemonic)).toBe(true);
        });
        
        test('should generate 24-word mnemonic', async () => {
            const mnemonic = wasmSdk.generate_mnemonic(24);
            const words = mnemonic.split(' ');
            
            console.log('  Generated 24-word mnemonic (first 10 words):', words.slice(0, 10).join(' ') + '...');
            
            expect(words.length).toBe(24);
            expect(wasmSdk.validate_mnemonic(mnemonic)).toBe(true);
        });
        
        test('should generate mnemonic in different languages', async () => {
            const languages = ['en', 'es', 'fr', 'it', 'ja', 'ko', 'pt', 'cs'];
            
            for (const lang of languages) {
                const mnemonic = wasmSdk.generate_mnemonic(12, lang);
                console.log(`  ${lang} mnemonic:`, mnemonic.substring(0, 40) + '...');
                expect(wasmSdk.validate_mnemonic(mnemonic, lang)).toBe(true);
            }
        });
    });
    
    describe('DIP13 paths', () => {
        test('should create correct DIP13 mainnet path info', () => {
            const result = wasmSdk.derivation_path_dip13_mainnet(0);
            
            console.log('  DIP13 mainnet path:', JSON.stringify(result, null, 2));
            
            expect(result).toBeDefined();
            expect(result.path).toBe("m/9'/5'/0'");
            expect(result.purpose).toBe(9);
            expect(result.coin_type).toBe(5);
            expect(result.account).toBe(0);
        });
        
        test('should create correct DIP13 testnet path info', () => {
            const result = wasmSdk.derivation_path_dip13_testnet(0);
            
            console.log('  DIP13 testnet path:', JSON.stringify(result, null, 2));
            
            expect(result).toBeDefined();
            expect(result.path).toBe("m/9'/1'/0'");
            expect(result.purpose).toBe(9);
            expect(result.coin_type).toBe(1);
            expect(result.account).toBe(0);
        });
    });
});

// Run tests
console.log('\nRunning tests...\n');
let passed = 0;
let failed = 0;

for (const { suite, name, fn } of tests) {
    try {
        await fn();
        console.log(`  ✅ ${name}`);
        passed++;
    } catch (error) {
        console.log(`  ❌ ${name}`);
        console.log(`     ${error.message}`);
        failed++;
    }
}

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${tests.length} total`);
process.exit(failed > 0 ? 1 : 0);