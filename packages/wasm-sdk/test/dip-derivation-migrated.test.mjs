#!/usr/bin/env node
// dip-derivation-migrated.test.mjs - DIP derivation tests using JavaScript wrapper (MIGRATED)

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) {
    Object.defineProperty(global, 'crypto', { value: webcrypto, writable: true, configurable: true });
}

// 🎯 MIGRATED: Import JavaScript wrapper
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

const sdk = new WasmSDK({ network: 'mainnet', proofs: false, debug: false });
await sdk.initialize();

let passed = 0, failed = 0;

async function test(name, fn) {
    try {
        await fn();
        console.log(`✅ ${name}`);
        passed++;
    } catch (error) {
        console.log(`❌ ${name}: ${error.message}`);
        failed++;
    }
}

const TEST_MNEMONIC = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

console.log('\n🎯 DIP Derivation Tests Using JavaScript Wrapper (MIGRATED)\n');

await test('DIP9 Identity key derivation', async () => {
    const path = "m/9'/5'/0'/0/0";
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, '', path, "mainnet"); // 🎯 MIGRATED
    if (!result.address || !result.private_key_wif) throw new Error('Missing key data');
    console.log(`   ✓ Identity key: ${result.address}`);
});

await test('DIP9 Authentication key derivation', async () => {
    const authPath = "m/9'/5'/3'/0";  
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, '', authPath, "mainnet"); // 🎯 MIGRATED
    if (!result.address) throw new Error('Missing address');
    console.log(`   ✓ Auth key: ${result.address}`);
});

await test('DIP9 Funding key derivation', async () => {
    const fundingPath = "m/44'/5'/1'/0";
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, '', fundingPath, "mainnet"); // 🎯 MIGRATED
    if (!result.address) throw new Error('Missing address');
});

await test('DIP9 TopUp key derivation', async () => {
    const topUpPath = "m/9'/5'/5'/0";
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, '', topUpPath, "mainnet"); // 🎯 MIGRATED
    if (!result.address) throw new Error('Missing address');
});

await test('DIP9 Invite key derivation', async () => {
    const invitePath = "m/9'/5'/4'/0"; 
    const result = await sdk.deriveKeyFromSeedWithPath(TEST_MNEMONIC, '', invitePath, "mainnet"); // 🎯 MIGRATED
    if (!result.address) throw new Error('Missing address');
});

await sdk.destroy();

console.log(`\n🎯 DIP-DERIVATION MIGRATION: ✅ ${passed} passed, ❌ ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);