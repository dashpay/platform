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

// Import JavaScript wrapper (correct approach)
import init, * as wasmSdk from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

async function runTests() {
    console.log('Testing DIP14 Test Vectors...\n');
    
    // Initialize WASM
    const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    await init(wasmBuffer);
    
    // Test helper
    let passed = 0;
    let failed = 0;
    
    const testVector = async (name, path, expectedXprv, expectedXpub) => {
        try {
            const seed = "b16d3782e714da7c55a397d5f19104cfed7ffa8036ac514509bbb50807f8ac598eeb26f0797bd8cc221a6cbff2168d90a5e9ee025a5bd977977b9eccd97894bb";
            const mnemonic = "birth kingdom trash renew flavor utility donkey gasp regular alert pave layer";
            
            console.log(`\n=== ${name} ===`);
            console.log(`Path: ${path}`);
            
            const result = await wasmSdk.derive_key_from_seed_with_extended_path(
                mnemonic,
                null,
                path,
                'testnet'
            );
            
            console.log('\nResult:');
            console.log(`xprv: ${result.xprv}`);
            console.log(`xpub: ${result.xpub}`);
            
            console.log('\nExpected:');
            console.log(`xprv: ${expectedXprv}`);
            console.log(`xpub: ${expectedXpub}`);
            
            const xprvMatch = result.xprv === expectedXprv;
            const xpubMatch = result.xpub === expectedXpub;
            
            console.log('\nComparison:');
            console.log(`xprv Match: ${xprvMatch ? '✅' : '❌'}`);
            console.log(`xpub Match: ${xpubMatch ? '✅' : '❌'}`);
            
            if (xprvMatch && xpubMatch) {
                console.log(`✅ ${name} PASSED`);
                passed++;
            } else {
                console.log(`❌ ${name} FAILED`);
                failed++;
            }
            
        } catch (error) {
            console.error(`❌ ${name} ERROR: ${error.message}`);
            failed++;
        }
    };
    
    // Test Vector 1: Non-hardened / Hardened path example
    await testVector(
        "Test Vector 1",
        "m/0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3b/0xf537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6'/0x4c4592ca670c983fc43397dfd21a6f427fac9b4ac53cb4dcdc6522ec51e81e79/0",
        "tprv8iNr6Z8PgAHmYSgMKGbq42kMVAAQmwmzm5iTJdUXoxLf25zG3GeRCvnEdC6HKTHkU59nZkfjvcGk9VW2YHsFQMwsZrQLyNrGx9c37kgb368",
        "tpubDF4tEyAdpXySRui9CvGRTSQU4BgLwGxuLPKEb9WqEE93raF2ffU1PRQ6oJHCgZ7dArzcMj9iKG8s8EFA1DdwgzWAXs61uFuRE1bQi8kAmLy"
    );
    
    // Test Vector 2: Multiple hardened derivations with final non-hardened index
    await testVector(
        "Test Vector 2",
        "m/9'/5'/15'/0'/0x555d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3a'/0xa137439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89b5'/0",
        "tprv8p9LqE2tA2b94gc3ciRNA525WVkFvzkcC9qjpKEcGaTqjb9u2pwTXj41KkZTj3c1a6fJUpyXRfcB4dimsYsLMjQjsTJwi5Ukx6tJ5BpmYpx",
        "tpubDLqNye58JQGox9dqWN5xZUgC5XGC6KwWmTSX6qGugrGEa5QffDm3iDfsVtX7qyXuWoQsXA6YCSuckKshyjnwiGGoYWHonAv2X98HTU613UH"
    );
    
    // Test Vector 3: Non-hardened derivation (note: these use DIP14 extended format)
    await testVector(
        "Test Vector 3",
        "m/0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3b",
        "dpts1vgMVEs9mmv1YLwURCeoTn9CFMZ8JMVhyZuxQSKttNSETR3zydMFHMKTTNDQPf6nnupCCtcNnSu3nKZXAJhaguyoJWD4Ju5PE6PSkBqAKWci7HLz37qmFmZZU6GMkLvNLtST2iV8NmqqbX37c45",
        "dptp1C5gGd8NzvAke5WNKyRfpDRyvV2UZ3jjrZVZU77qk9yZemMGSdZpkWp7y6wt3FzvFxAHSW8VMCaC1p6Ny5EqWuRm2sjvZLUUFMMwXhmW6eS69qjX958RYBH5R8bUCGZkCfUyQ8UVWcx9katkrRr"
    );
    
    // Test Vector 4: Hardened path with complex indices
    await testVector(
        "Test Vector 4",
        "m/0x775d3854c910b7dee436869c4724bed2fe0784e198b8a39f02bbb49d8ebcfc3b/0xf537439f36d04a15474ff7423e4b904a14373fafb37a41db74c84f1dbb5c89a6'",
        "dpts1vwRsaPMQfqwp59ELpx5UeuYtdaMCJyGTwiGtr8zgf6qWPMWnhPpg8R73hwR1xLibbdKVdh17zfwMxFEMxZzBKUgPwvuosUGDKW4ayZjs3AQB9EGRcVpDoFT8V6nkcc6KzksmZxvmDcd3MqiPEu",
        "dptp1CLkexeadp6guoi8Fbiwq6CLZm3hT1DJLwHsxWvwYSeAhjenFhcQ9HumZSftfZEr4dyQjFD7gkM5bSn6Aj7F1Jve8KTn4JsMEaj9dFyJkYs4Ga5HSUqeajxGVmzaY1pEioDmvUtZL3J1NCDCmzQ"
    );
    
    // Summary
    console.log('\n\n=== Test Summary ===');
    console.log(`Passed: ${passed}`);
    console.log(`Failed: ${failed}`);
    console.log(`Total: ${passed + failed}`);
    
    console.log('\n\n=== Notes ===');
    console.log('- Test vectors 1 & 2 use standard BIP32 format (tprv/tpub)');
    console.log('- Test vectors 3 & 4 use DIP14 extended format (dpts/dptp)');
    console.log('- The DIP14 format is used when any index >= 2^32');
    console.log('- Our implementation currently only supports standard BIP32 serialization');
    
    process.exit(failed > 0 ? 1 : 0);
}

runTests().catch(console.error);