#!/usr/bin/env node
// Test DIP14 256-bit derivation with test vector 2

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
import init, * as wasmSdk from './pkg/wasm_sdk.js';

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, './pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Test Vector 2
const testVector = {
    mnemonic: "birth kingdom trash renew flavor utility donkey gasp regular alert pave layer",
    seedHex: "b16d3782e714da7c55a397d5f19104cfed7ffa8036ac514509bbb50807f8ac598eeb26f0797bd8cc221a6cbff2168d90a5e9ee025a5bd977977b9eccd97894bb",
    masterHD: {
        xprv: "tprv8ZgxMBicQKsPeTb4MhYiJKST5uCW8dQ2swMcH9rAv9JPdadYK9LKCcdKd8a2FopTjKH9rvw8rELFpJSKCEV6pzVLmNUVFGKvwN1Y8WqhSoZ",
        xpub: "tpubD6NzVbkMyxFKPuttqg8FFuNrJK7TiVVwKygWzcdDmLbtKo3F1cvZxtukxAirHbQLDG4pCkc4FGxgpVw3zPaRMDDugf3e8NVFHiYMnpmn3Bg"
    },
    path: "m/0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a/0xf537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6'",
    expected: {
        privateKey: "0xfac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60",
        xprv: "tprv8p9LqE2tA2b94gc3ciRNA525WVkFvzkcC9qjpKEcGaTqjb9u2pwTXj41KkZTj3c1a6fJUpyXRfcB4dimsYsLMjQjsTJwi5Ukx6tJ5BpmYpx",
        xpub: "tpubDKZGWTkDtRpjWxsJ2qGVGNNq33aL8ji9Dz6ndDK46BQagP2kByTCFiQYu9fkBwCBrKNhCk7pL9ysjdtcaqAXEQNHDCZY8iXN6YAdq1qecKN"
    }
};

console.log('\n=== DIP14 Test Vector 2 ===\n');

// First, verify the seed generation
const seedBytes = wasmSdk.mnemonic_to_seed(testVector.mnemonic, null);
const seed = Buffer.from(seedBytes).toString('hex');
console.log('Generated seed:', seed);
console.log('Expected seed: ', testVector.seedHex);
console.log('Seed match:', seed === testVector.seedHex ? '✅' : '❌');

// Try to derive using the extended path
console.log('\nDeriving key with path:', testVector.path);

try {
    const result = wasmSdk.derive_key_from_seed_with_extended_path(
        testVector.mnemonic,
        null,
        testVector.path,
        "testnet"
    );
    
    console.log('\nDerived values:');
    console.log('Private key:', result.private_key_hex);
    console.log('Expected:   ', testVector.expected.privateKey.slice(2)); // Remove 0x prefix
    console.log('Match:', result.private_key_hex === testVector.expected.privateKey.slice(2) ? '✅' : '❌');
    
    console.log('\nExtended private key:', result.xprv);
    console.log('Expected:            ', testVector.expected.xprv);
    console.log('Match:', result.xprv === testVector.expected.xprv ? '✅' : '❌');
    
    console.log('\nExtended public key:', result.xpub);
    console.log('Expected:           ', testVector.expected.xpub);
    console.log('Match:', result.xpub === testVector.expected.xpub ? '✅' : '❌');
    
    // Analyze what's happening
    console.log('\n=== Analysis ===');
    console.log('Path components:');
    console.log('1. Master key: m');
    console.log('2. First ID:  0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a (256 bits)');
    console.log('3. Second ID: 0xf537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6\' (256 bits, hardened)');
    
    // Try to understand what index is being used
    const firstIdBytes = Buffer.from('775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a', 'hex');
    const firstFourBytes = firstIdBytes.slice(0, 4);
    const index1 = firstFourBytes.readUInt32BE(0);
    console.log('\nFirst ID first 4 bytes:', firstFourBytes.toString('hex'));
    console.log('As u32 (BE):', index1);
    console.log('As u32 (BE) & 0x7FFFFFFF:', index1 & 0x7FFFFFFF);
    
    const secondIdBytes = Buffer.from('f537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6', 'hex');
    const secondFourBytes = secondIdBytes.slice(0, 4);
    const index2 = secondFourBytes.readUInt32BE(0);
    console.log('\nSecond ID first 4 bytes:', secondFourBytes.toString('hex'));
    console.log('As u32 (BE):', index2);
    console.log('As u32 (BE) & 0x7FFFFFFF:', index2 & 0x7FFFFFFF);
    
} catch (error) {
    console.error('Error deriving key:', error.message);
}

// Also try deriving the master key manually
console.log('\n=== Master Key Verification ===');
try {
    const masterResult = wasmSdk.derive_key_from_seed_with_path(
        testVector.mnemonic,
        null,
        "m",
        "testnet"
    );
    
    console.log('Master xprv:', masterResult.xprv);
    console.log('Expected:   ', testVector.masterHD.xprv);
    console.log('Match:', masterResult.xprv === testVector.masterHD.xprv ? '✅' : '❌');
    
    console.log('\nMaster xpub:', masterResult.xpub);
    console.log('Expected:   ', testVector.masterHD.xpub);
    console.log('Match:', masterResult.xpub === testVector.masterHD.xpub ? '✅' : '❌');
} catch (error) {
    console.error('Error deriving master key:', error.message);
}

process.exit(0);