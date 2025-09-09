#!/usr/bin/env node

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

async function runTest() {
    console.log('Testing DIP14 256-bit Derivation Implementation...\n');
    
    // Initialize WASM
    const wasmPath = join(__dirname, 'pkg/wasm_sdk_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    await init(wasmBuffer);
    
    // Test Vector 2 from DIP14
    console.log('=== DIP14 Test Vector 2 ===\n');
    
    const testMnemonic = "birth kingdom trash renew flavor utility donkey gasp regular alert pave layer";
    const testPath = "m/9'/5'/15'/0'/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0";
    
    // Expected results from test vector
    const expected = {
        privateKey: "0xfac40790776d171ee1db90899b5eb2df2f7d2aaf35ad56f07ffb8ed2c57f8e60",
        xprv: "tprv8p9LqE2tA2b94gc3ciRNA525WVkFvzkcC9qjpKEcGaTqjb9u2pwTXj41KkZTj3c1a6fJUpyXRfcB4dimsYsLMjQjsTJwi5Ukx6tJ5BpmYpx",
        xpub: "tpubDKZGWTkDtRpjWxsJ2qGVGNNq33aL8ji9Dz6ndDK46BQagP2kByTCFiQYu9fkBwCBrKNhCk7pL9ysjdtcaqAXEQNHDCZY8iXN6YAdq1qecKN"
    };
    
    try {
        // Test using extended derivation
        console.log('Testing with derive_key_from_seed_with_extended_path...');
        const result = await wasmSdk.derive_key_from_seed_with_extended_path(
            testMnemonic, 
            null, 
            testPath, 
            'testnet'
        );
        
        console.log('\nResult:');
        console.log('Path:', result.path);
        console.log('Private Key:', result.private_key_hex);
        console.log('Extended Private Key:', result.xprv);
        console.log('Extended Public Key:', result.xpub);
        console.log('Address:', result.address);
        
        console.log('\nExpected:');
        console.log('Private Key:', expected.privateKey);
        console.log('Extended Private Key:', expected.xprv);
        console.log('Extended Public Key:', expected.xpub);
        
        console.log('\nComparison:');
        console.log('Private Key Match:', result.private_key_hex === expected.privateKey.slice(2));
        console.log('xprv Match:', result.xprv === expected.xprv);
        console.log('xpub Match:', result.xpub === expected.xpub);
        
        // Also test the dashpay contact key function
        console.log('\n\nTesting with derive_dashpay_contact_key...');
        const contactResult = await wasmSdk.derive_dashpay_contact_key(
            testMnemonic,
            null,
            "0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a",
            "0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5",
            0,
            0,
            'testnet'
        );
        
        console.log('\nContact Key Result:');
        console.log('Path:', contactResult.path);
        console.log('Private Key:', contactResult.private_key_hex);
        console.log('Extended Private Key:', contactResult.xprv);
        console.log('Extended Public Key:', contactResult.xpub);
        
    } catch (error) {
        console.error('Error:', error.message);
    }
    
    process.exit(0);
}

runTest().catch(console.error);