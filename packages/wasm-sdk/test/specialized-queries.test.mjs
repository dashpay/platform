#!/usr/bin/env node
// specialized-queries.test.mjs - Tests for masternode, group, and other specialized queries

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

console.log('\nSpecialized Query Tests\n');

// Initialize SDK - use trusted builder for WASM
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Masternode Queries
describe('Masternode Queries');

await test('get_masternode_status - requires valid proTxHash', async () => {
    try {
        // This would need a real masternode proTxHash
        const result = await wasmSdk.get_masternode_status(
            sdk,
            "0000000000000000000000000000000000000000000000000000000000000000" // 32-byte hex
        );
        throw new Error('Should have failed with invalid proTxHash');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid proTxHash');
    }
});

await test('get_masternode_score - requires valid proTxHash', async () => {
    try {
        const result = await wasmSdk.get_masternode_score(
            sdk,
            "0000000000000000000000000000000000000000000000000000000000000000",
            1 // quorum count
        );
        throw new Error('Should have failed with invalid proTxHash');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid proTxHash');
    }
});

await test('get_evonodes - fetch evonode list', async () => {
    try {
        const result = await wasmSdk.get_evonodes(
            sdk,
            10,   // limit
            null, // no start after
            null  // no start at
        );
        console.log(`   Found ${result?.length || 0} evonodes`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Group Queries
describe('Group Queries');

await test('group_query_public_keys_and_identity_ids - requires group definition', async () => {
    try {
        // Example group query for potential proposers
        const groupQuery = JSON.stringify({
            proposers: {
                query_type: "last_quorum_hash",
                quorum_type: 4,
                count: 60
            }
        });
        
        const result = await wasmSdk.group_query_public_keys_and_identity_ids(
            sdk,
            groupQuery,
            true // prove
        );
        console.log(`   Group query completed`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Proof Verification Queries
describe('Proof Verification');

await test('get_proofs_query - multiple query proofs', async () => {
    try {
        // Example: Get proofs for multiple queries
        const query1 = {
            identity_fetch: {
                identity_id: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
            }
        };
        
        const query2 = {
            data_contract_fetch: {
                contract_id: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
            }
        };
        
        const queries = JSON.stringify([query1, query2]);
        
        const result = await wasmSdk.get_proofs_query(sdk, queries);
        console.log(`   Proofs query completed`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Specialized Token Queries
describe('Specialized Token Queries');

await test('get_token_infos_by_owner - list all tokens owned by identity', async () => {
    try {
        const result = await wasmSdk.get_token_infos_by_owner(
            sdk,
            "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            10,   // limit
            null, // no start after
            null  // no start at
        );
        console.log(`   Found ${result?.length || 0} tokens owned by identity`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_total_token_supplies - get supplies for multiple tokens', async () => {
    try {
        const result = await wasmSdk.get_total_token_supplies(
            sdk,
            ["Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"] // token contract IDs
        );
        console.log(`   Token supplies: ${JSON.stringify(result)}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Metadata Queries
describe('Metadata Queries');

await test('get_current_protocol_version', async () => {
    try {
        const result = await wasmSdk.get_current_protocol_version(sdk);
        console.log(`   Current protocol version: ${result}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

await test('get_platform_block_metadata', async () => {
    try {
        const result = await wasmSdk.get_platform_block_metadata(sdk);
        console.log(`   Platform block height: ${result?.height || 'N/A'}`);
        console.log(`   Core chain locked height: ${result?.coreChainLockedHeight || 'N/A'}`);
    } catch (error) {
        if (error.message.includes('network') || error.message.includes('connection')) {
            console.log('   Expected network error (offline)');
        } else {
            throw error;
        }
    }
});

// Specialized Document Queries  
describe('Specialized Document Queries');

await test('get_prefunded_specialized_balance - requires valid document ID', async () => {
    try {
        const result = await wasmSdk.get_prefunded_specialized_balance(
            sdk,
            "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec", // contract ID
            "invalidDocumentId"
        );
        throw new Error('Should have failed with invalid document ID');
    } catch (error) {
        if (error.message.includes('Should have failed')) {
            throw error;
        }
        console.log('   Expected error with invalid document ID');
    }
});

await test('get_total_document_count_in_system', async () => {
    try {
        const result = await wasmSdk.get_total_document_count_in_system(sdk);
        console.log(`   Total documents in system: ${result}`);
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
console.log('- Many specialized queries require valid on-chain data (proTxHash, document IDs, etc.)');
console.log('- Network errors are expected when running offline');
console.log('- Group queries require specific quorum configurations');
console.log('- Some queries may be restricted or require special permissions');

process.exit(failed > 0 ? 1 : 0);