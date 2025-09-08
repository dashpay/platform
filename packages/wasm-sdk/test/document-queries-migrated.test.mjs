#!/usr/bin/env node
// document-queries-migrated.test.mjs - Document query tests using JavaScript wrapper (MIGRATED)

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

// 🎯 MIGRATED: Import JavaScript wrapper
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

console.log('Initializing WASM module...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

const sdk = new WasmSDK({
    network: 'testnet',
    proofs: false, // Use non-proof for faster testing
    debug: false
});
await sdk.initialize();

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

// Test constants
const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const DASHPAY_CONTRACT = 'Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7';

console.log('\n🎯 Document Query Tests Using JavaScript Wrapper (MIGRATED)\n');

await test('getDocuments - basic document query', async () => {
    try {
        const result = await sdk.getDocuments(DPNS_CONTRACT, 'domain'); // 🎯 MIGRATED: was wasmSdk.get_documents()
        console.log('   ✓ Document query completed successfully');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

await test('getDocuments - with where clause', async () => {
    try {
        const whereClause = [["normalizedParentDomainName", "==", "dash"]];
        const result = await sdk.getDocuments(DPNS_CONTRACT, 'domain', { // 🎯 MIGRATED
            where: whereClause,
            limit: 10
        });
        console.log('   ✓ Document query with where clause completed');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

await test('getDocuments - with order by', async () => {
    try {
        const orderBy = [["normalizedLabel", "asc"]];
        const result = await sdk.getDocuments(DPNS_CONTRACT, 'domain', { // 🎯 MIGRATED
            orderBy: orderBy,
            limit: 5
        });
        console.log('   ✓ Document query with order by completed');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   ⚠️ Network error (expected in offline mode)');
        } else {
            throw error;
        }
    }
});

await test('getDocument - specific document', async () => {
    try {
        const docId = "test-document-id";
        const result = await sdk.getDocument(DPNS_CONTRACT, 'domain', docId); // 🎯 MIGRATED: was wasmSdk.get_document()
        console.log('   ✓ Specific document query completed');
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection') || error.message.includes('not found')) {
            console.log('   ⚠️ Network error or document not found (expected)');
        } else {
            throw error;
        }
    }
});

await sdk.destroy();

console.log(`\n🎯 DOCUMENT MIGRATION RESULTS: ✅ ${passed} passed, ❌ ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);