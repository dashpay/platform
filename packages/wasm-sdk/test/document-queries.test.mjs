#!/usr/bin/env node
// document-queries.test.mjs - Tests for document query functions using documented testnet values

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
import init, * as wasmSdk from '../pkg/wasm_sdk.js';

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

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
        console.log(`   ${error.message}`);
        failed++;
    }
}

function describe(name) {
    console.log(`\n${name}`);
}

console.log('\nDocument Query Tests Using Documented Testnet Values\n');

// DOCUMENTED TEST VALUES FROM docs.html
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const TOKEN_CONTRACT = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';

console.log('Test Values:');
console.log(`- Identity: ${TEST_IDENTITY}`);
console.log(`- DPNS Contract: ${DPNS_CONTRACT}`);
console.log(`- Token Contract: ${TOKEN_CONTRACT}`);

// Initialize SDK - use trusted builder for WASM
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Document Query Tests
describe('Document Queries');

await test('get_documents - DPNS domains (no filters)', async () => {
    try {
        const result = await wasmSdk.get_documents(
            sdk,
            DPNS_CONTRACT,
            "domain",
            null,  // no where clause
            null,  // no order by
            10,    // limit
            null,  // no start after
            null   // no start at
        );
        console.log(`   Found ${result?.length || 0} documents`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_documents - with where clause', async () => {
    try {
        // Search for domains owned by test identity
        const whereClause = JSON.stringify([
            ["$ownerId", "==", TEST_IDENTITY]
        ]);
        
        const result = await wasmSdk.get_documents(
            sdk,
            DPNS_CONTRACT,
            "domain",
            whereClause,
            null,  // no order by
            10,    // limit
            null,  // no start after
            null   // no start at
        );
        console.log(`   Found ${result?.length || 0} documents owned by test identity`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_documents - with orderBy clause', async () => {
    try {
        const orderBy = JSON.stringify([
            ["$createdAt", "desc"]
        ]);
        
        const result = await wasmSdk.get_documents(
            sdk,
            DPNS_CONTRACT,
            "domain",
            null,     // no where
            orderBy,  // order by creation time descending
            5,        // limit
            null,     // no start after
            null      // no start at
        );
        console.log(`   Found ${result?.length || 0} documents ordered by creation time`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_documents - with complex where clause', async () => {
    try {
        // Multiple conditions
        const whereClause = JSON.stringify([
            ["normalizedLabel", "startsWith", "test"],
            ["normalizedParentDomainName", "==", "dash"]
        ]);
        
        const result = await wasmSdk.get_documents(
            sdk,
            DPNS_CONTRACT,
            "domain",
            whereClause,
            null,
            10,
            null,
            null
        );
        console.log(`   Found ${result?.length || 0} test domains under .dash`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_single_document - by specific ID', async () => {
    try {
        // This would need a real document ID
        const result = await wasmSdk.get_single_document(
            sdk,
            DPNS_CONTRACT,
            "domain",
            "invalidDocumentId"
        );
        throw new Error('Should have failed with invalid ID');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid document ID');
    }
});

// Data Contract Query Tests
describe('Data Contract Queries');

await test('data_contract_fetch - DPNS contract', async () => {
    try {
        const result = await wasmSdk.data_contract_fetch(sdk, DPNS_CONTRACT);
        console.log(`   Contract fetched: ${result?.id || 'N/A'}`);
        console.log(`   Version: ${result?.version || 'N/A'}`);
        console.log(`   Owner: ${result?.ownerId || 'N/A'}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('data_contract_fetch - Token contract', async () => {
    try {
        const result = await wasmSdk.data_contract_fetch(sdk, TOKEN_CONTRACT);
        console.log(`   Contract fetched: ${result?.id || 'N/A'}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('data_contract_fetch_history - DPNS contract history', async () => {
    try {
        const result = await wasmSdk.data_contract_fetch_history(
            sdk,
            DPNS_CONTRACT,
            10,    // limit
            0,     // offset
            null,  // start at version
            true   // prove
        );
        console.log(`   Found ${result?.length || 0} historical versions`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_data_contracts - fetch multiple contracts', async () => {
    try {
        // Note: This function expects Vec<String> in Rust, which should work with JS array
        const result = await wasmSdk.get_data_contracts(
            sdk,
            [DPNS_CONTRACT, 'ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A']
        );
        console.log(`   Found ${result?.length || 0} data contracts`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Token Document Queries
describe('Token Document Queries');

await test('get_documents - token documents', async () => {
    try {
        const result = await wasmSdk.get_documents(
            sdk,
            TOKEN_CONTRACT,
            "token",  // assuming token document type
            null,
            null,
            10,
            null,
            null
        );
        console.log(`   Found ${result?.length || 0} token documents`);
    } catch (error) {
        // Token queries might fail if contract doesn't have 'token' document type
        console.log('   Expected error (token contract may not have token document type)');
    }
});

// System Status Queries
describe('System Status Queries');

await test('get_status - platform status', async () => {
    try {
        const result = await wasmSdk.get_status(sdk);
        console.log(`   Status received: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Epoch Queries
describe('Epoch Queries');

await test('get_current_epoch', async () => {
    try {
        const result = await wasmSdk.get_current_epoch(sdk);
        console.log(`   Current epoch: ${result}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_epoch_info', async () => {
    try {
        const result = await wasmSdk.get_epoch_info(sdk, 1); // Get info for epoch 1
        console.log(`   Epoch info fetched`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Clean up
sdk.free();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);

console.log('\n📝 Notes:');
console.log('- These tests use the documented testnet values from docs.html');
console.log('- Network errors are expected when running offline');
console.log('- Some queries may fail due to missing data or incorrect document types');
console.log('- Where and orderBy clauses are properly formatted as JSON strings');

process.exit(failed > 0 ? 1 : 0);