#!/usr/bin/env node
/**
 * Key Index Authentication Test - Find correct keyIndex for funded identity
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

await init(readFileSync(join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm')));

console.log('🔍 KEY INDEX AUTHENTICATION TEST');
console.log('Finding correct keyIndex for funded identity\\n');

const sdk = new WasmSDK({ network: 'testnet', proofs: false, debug: true });
await sdk.initialize();

const IDENTITY_ID = process.env.IDENTITY_ID || 'DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq';
const MNEMONIC = process.env.MNEMONIC || 'lamp truck drip furnace now swing income victory leisure popular jeans vehicle';

console.log('Testing identity:', IDENTITY_ID);
console.log('Using mnemonic:', MNEMONIC.split(' ').slice(0, 3).join(' ') + '...');

// Test different key indexes
for (let keyIndex = 0; keyIndex < 5; keyIndex++) {
    console.log(`\\n🔑 Testing keyIndex ${keyIndex}:`);
    
    try {
        const result = await sdk.createDocument(
            MNEMONIC,
            IDENTITY_ID,
            'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            'domain',
            JSON.stringify({
                label: `keytest${keyIndex}${Date.now()}`,
                normalizedLabel: `keytest${keyIndex}${Date.now()}`,
                parentDomainName: 'dash'
            }),
            keyIndex
        );
        
        console.log(`🎉 SUCCESS with keyIndex ${keyIndex}!`);
        console.log('Result:', result);
        console.log('💰 REAL CREDIT CONSUMPTION ACHIEVED!');
        break;
        
    } catch (error) {
        if (error.message && error.message.includes('No matching authentication key')) {
            console.log(`   ❌ KeyIndex ${keyIndex}: Authentication key mismatch`);
        } else if (error.message && error.message.includes('Identity not found')) {
            console.log(`   ❌ KeyIndex ${keyIndex}: Identity lookup failed`);
        } else {
            console.log(`   ❌ KeyIndex ${keyIndex}: ${error.message || error.toString()}`);
        }
    }
}

await sdk.destroy();
console.log('\\n✅ Key index authentication test complete');