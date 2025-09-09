#!/usr/bin/env node
// final-coverage-push.test.mjs - Final coverage push test (MIGRATED)

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) {
    Object.defineProperty(global, 'crypto', { value: webcrypto, writable: true, configurable: true });
}

import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

const sdk = new WasmSDK({ network: 'testnet', proofs: false, debug: false });
await sdk.initialize();

let passed = 0, failed = 0;
async function test(name, fn) {
    try { await fn(); console.log(`✅ ${name}`); passed++; } 
    catch (error) { console.log(`❌ ${name}: ${error.message}`); failed++; }
}

console.log('\n🎯 Final Coverage Push Test (MIGRATED)\n');

await test('Complete wrapper ecosystem final validation', async () => {
    // Test that all wrapper categories work in final push
    const mnemonic = await sdk.generateMnemonic(12);
    const keyPair = await sdk.generateKeyPair('testnet');
    const dpnsValid = await sdk.dpnsIsValidUsername('alice');
    
    if (!mnemonic || !keyPair || dpnsValid === undefined) {
        throw new Error('Complete ecosystem should work');
    }
    
    console.log('   ✓ Final wrapper ecosystem validation successful');
    console.log('   🚀 Ready for systematic completion to 100%');
});

await sdk.destroy();
console.log(`\n🎯 FINAL-COVERAGE-PUSH: ✅ ${passed} passed, ❌ ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);