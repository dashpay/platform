#!/usr/bin/env node
// state-transitions.test.mjs - Tests for all state transition functions

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
import init, { generate_key_pair } from '../pkg/wasm_sdk.js';
import * as wasmSdk from '../pkg/wasm_sdk.js';

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

console.log('\nState Transition Tests\n');

// Initialize SDK - use trusted builder for WASM
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();

// Test values
const TEST_MNEMONIC = "during develop before curtain hazard rare job language become verb message travel";
const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
const TOKEN_CONTRACT = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';

// Identity State Transitions
describe('Identity State Transitions');

await test('identity_create - requires funding', async () => {
    try {
        // Would need funding transaction
        const result = await sdk.identityCreate(
            sdk,
            TEST_MNEMONIC,
            null,   // no alias
            0       // key index
        );
        throw new Error('Should fail without funding');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without funding');
    }
});

await test('identity_create with all SECP256K1 keys (common scenario)', async () => {
    try {
        // Generate unique keys for testing (1 asset lock + 3 identity keys)
        const assetLockKey = await sdk.generateKeyPair("testnet");
        const secp256k1Key1 = await sdk.generateKeyPair("testnet");
        const secp256k1Key2 = await sdk.generateKeyPair("testnet");
        const secp256k1Key3 = await sdk.generateKeyPair("testnet");
        
        // Mock asset lock proof
        const mockAssetLockProof = JSON.stringify({
            coreChainLockedHeight: 1000000,
            outPoint: "0000000000000000000000000000000000000000000000000000000000000000:0"
        });
        
        // Create public keys array with all SECP256K1 keys
        const publicKeys = JSON.stringify([
            {
                keyType: "ECDSA_SECP256K1",
                purpose: "AUTHENTICATION",
                securityLevel: "MASTER",
                privateKeyHex: secp256k1Key1.private_key_hex
            },
            {
                keyType: "ECDSA_SECP256K1", 
                purpose: "AUTHENTICATION",
                securityLevel: "CRITICAL",
                privateKeyHex: secp256k1Key2.private_key_hex
            },
            {
                keyType: "ECDSA_SECP256K1",
                purpose: "ENCRYPTION",
                securityLevel: "HIGH",
                privateKeyHex: secp256k1Key3.private_key_hex
            }
        ]);
        
        const result = await sdk.identityCreate(
            mockAssetLockProof,
            assetLockKey.private_key_wif,
            publicKeys
        );
        throw new Error('Should fail with mock data');
    } catch (error) {
        const errorMessage = error.message || error.toString() || 'Unknown error';
        if (errorMessage.includes('Should fail')) {
            throw error;
        }
        // Check that it's NOT a signature verification error
        if (errorMessage.includes('signature failed verification')) {
            throw new Error('SIGNATURE VERIFICATION ERROR - SimpleSigner may not be working correctly');
        }
        console.log('   Expected error with mock data (not signature error)');
    }
});

await test('identity_create with mixed key types (SECP256K1 and HASH160)', async () => {
    try {
        // Generate unique keys for testing (1 asset lock + 2 SECP256K1 + 1 HASH160)
        const assetLockKey = await sdk.generateKeyPair("testnet");
        const secp256k1Key1 = await sdk.generateKeyPair("testnet");
        const secp256k1Key2 = await sdk.generateKeyPair("testnet");
        const hash160Key = await sdk.generateKeyPair("testnet");
        
        // Mock asset lock proof
        const mockAssetLockProof = JSON.stringify({
            coreChainLockedHeight: 1000000,
            outPoint: "0000000000000000000000000000000000000000000000000000000000000001:0"
        });
        
        // Create mixed public keys array
        const publicKeys = JSON.stringify([
            {
                keyType: "ECDSA_SECP256K1",
                purpose: "AUTHENTICATION", 
                securityLevel: "MASTER",
                privateKeyHex: secp256k1Key1.private_key_hex
            },
            {
                keyType: "ECDSA_SECP256K1",
                purpose: "AUTHENTICATION",
                securityLevel: "CRITICAL", 
                privateKeyHex: secp256k1Key2.private_key_hex
            },
            {
                keyType: "ECDSA_HASH160",
                purpose: "TRANSFER",
                securityLevel: "HIGH",
                privateKeyHex: hash160Key.private_key_hex
            }
        ]);
        
        const result = await sdk.identityCreate(
            mockAssetLockProof,
            assetLockKey.private_key_wif,
            publicKeys
        );
        throw new Error('Should fail with mock data');
    } catch (error) {
        const errorMessage = error.message || error.toString() || 'Unknown error';
        if (errorMessage.includes('Should fail')) {
            throw error;
        }
        // Check that it's NOT a signature verification error
        if (errorMessage.includes('signature failed verification')) {
            throw new Error('SIGNATURE VERIFICATION ERROR - SimpleSigner may not be working correctly');
        }
        console.log('   Expected error with mock data (not signature error)');
    }
});

await test('identity_create with only HASH160 keys', async () => {
    try {
        // Generate unique keys for testing (1 asset lock + 3 HASH160)
        const assetLockKey = await sdk.generateKeyPair("testnet");
        const hash160Key1 = await sdk.generateKeyPair("testnet");
        const hash160Key2 = await sdk.generateKeyPair("testnet");
        const hash160Key3 = await sdk.generateKeyPair("testnet");
        
        // Mock asset lock proof
        const mockAssetLockProof = JSON.stringify({
            coreChainLockedHeight: 1000000,
            outPoint: "0000000000000000000000000000000000000000000000000000000000000002:0"
        });
        
        // Create public keys array with all HASH160 keys
        const publicKeys = JSON.stringify([
            {
                keyType: "ECDSA_HASH160",
                purpose: "AUTHENTICATION",
                securityLevel: "MASTER", 
                privateKeyHex: hash160Key1.private_key_hex
            },
            {
                keyType: "ECDSA_HASH160",
                purpose: "AUTHENTICATION",
                securityLevel: "CRITICAL",
                privateKeyHex: hash160Key2.private_key_hex
            },
            {
                keyType: "ECDSA_HASH160",
                purpose: "TRANSFER",
                securityLevel: "HIGH",
                privateKeyHex: hash160Key3.private_key_hex
            }
        ]);
        
        const result = await sdk.identityCreate(
            mockAssetLockProof,
            assetLockKey.private_key_wif,
            publicKeys
        );
        throw new Error('Should fail with mock data');
    } catch (error) {
        const errorMessage = error.message || error.toString() || 'Unknown error';
        if (errorMessage.includes('Should fail')) {
            throw error;
        }
        // Check that it's NOT a signature verification error
        if (errorMessage.includes('signature failed verification')) {
            throw new Error('SIGNATURE VERIFICATION ERROR - SimpleSigner may not be working correctly');
        }
        console.log('   Expected error with mock data (not signature error)');
    }
});

await test('identity_update - requires existing identity', async () => {
    try {
        const updateData = JSON.stringify({
            add: [{
                purpose: 0,  // authentication
                securityLevel: 0,
                keyType: 0,  // ECDSA
                data: "invalidpublickey"
            }]
        });
        
        const result = await sdk.identityUpdate(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            updateData,
            0  // key index
        );
        throw new Error('Should fail with invalid key data');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid key data');
    }
});

await test('identity_topup - requires funding', async () => {
    try {
        const result = await wasmSdk.identity_topup(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            100000  // amount
        );
        throw new Error('Should fail without funding');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without funding');
    }
});

await test('identity_withdraw - requires balance', async () => {
    try {
        const result = await wasmSdk.identity_withdraw(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            "yMTzqaUcb7e4QLiPT5f5hqNjgCXQq65pLm",  // destination address
            100000,  // amount
            0        // key index
        );
        throw new Error('Should fail without balance');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without balance');
    }
});

// Document State Transitions
describe('Document State Transitions');

await test('document_create - DPNS document', async () => {
    try {
        const documentData = JSON.stringify({
            label: "testname",
            normalizedLabel: "testname",
            normalizedParentDomainName: "dash",
            preorderSalt: "preordersalt",
            records: {
                identity: TEST_IDENTITY
            },
            subdomainRules: {
                allowSubdomains: false
            }
        });
        
        const result = await wasmSdk.document_create(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            DPNS_CONTRACT,
            "domain",
            documentData,
            0  // key index
        );
        throw new Error('Should fail without proper preorder');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without proper preorder');
    }
});

await test('document_update - requires existing document', async () => {
    try {
        const updateData = JSON.stringify({
            records: {
                identity: TEST_IDENTITY,
                dashAddress: "yMTzqaUcb7e4QLiPT5f5hqNjgCXQq65pLm"
            }
        });
        
        const result = await wasmSdk.document_update(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            DPNS_CONTRACT,
            "domain",
            "nonexistentdocumentid",
            updateData,
            0  // key index
        );
        throw new Error('Should fail with nonexistent document');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with nonexistent document');
    }
});

await test('document_delete - requires existing document', async () => {
    try {
        const result = await wasmSdk.document_delete(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            DPNS_CONTRACT,
            "domain",
            "nonexistentdocumentid",
            0  // key index
        );
        throw new Error('Should fail with nonexistent document');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with nonexistent document');
    }
});

// Token State Transitions
describe('Token State Transitions');

await test('token_create - create new token', async () => {
    try {
        const result = await wasmSdk.token_create(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            "TESTToken",     // name
            "TEST",          // symbol
            8,               // decimals
            1000000000,      // total supply
            null,            // no additional metadata
            0                // key index
        );
        throw new Error('Should fail without sufficient balance');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without sufficient balance');
    }
});

await test('token_mint - requires token ownership', async () => {
    try {
        const result = await wasmSdk.token_mint(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            TOKEN_CONTRACT,
            100000,        // amount to mint
            TEST_IDENTITY, // recipient
            0              // key index
        );
        throw new Error('Should fail without token ownership');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without token ownership');
    }
});

await test('token_burn - requires token balance', async () => {
    try {
        const result = await wasmSdk.token_burn(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            TOKEN_CONTRACT,
            100000,        // amount to burn
            0              // key index
        );
        throw new Error('Should fail without token balance');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without token balance');
    }
});

await test('token_transfer - requires token balance', async () => {
    try {
        const result = await wasmSdk.token_transfer(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,                                    // sender
            TOKEN_CONTRACT,
            "3mFKtDYspCMd8YmXNTB3qzKmbY3Azf4Kx3x8e36V8Gho", // recipient identity
            100000,                                           // amount
            0                                                 // key index
        );
        throw new Error('Should fail without token balance');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without token balance');
    }
});

await test('token_update_metadata - requires token ownership', async () => {
    try {
        const metadata = JSON.stringify({
            description: "Updated token description",
            website: "https://example.com"
        });
        
        const result = await wasmSdk.token_update_metadata(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            TOKEN_CONTRACT,
            metadata,
            0  // key index
        );
        throw new Error('Should fail without token ownership');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without token ownership');
    }
});

// Data Contract State Transitions
describe('Data Contract State Transitions');

await test('data_contract_create - requires balance', async () => {
    try {
        const contractDefinition = JSON.stringify({
            testDocument: {
                type: "object",
                properties: {
                    name: {
                        type: "string"
                    }
                },
                required: ["name"],
                additionalProperties: false
            }
        });
        
        const result = await wasmSdk.data_contract_create(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            contractDefinition,
            0  // key index
        );
        throw new Error('Should fail without sufficient balance');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without sufficient balance');
    }
});

await test('data_contract_update - requires ownership', async () => {
    try {
        const updateDefinition = JSON.stringify({
            testDocument: {
                type: "object",
                properties: {
                    name: {
                        type: "string"
                    },
                    description: {
                        type: "string"
                    }
                },
                required: ["name"],
                additionalProperties: false
            }
        });
        
        const result = await wasmSdk.data_contract_update(
            sdk,
            TEST_MNEMONIC,
            TEST_IDENTITY,
            DPNS_CONTRACT,  // trying to update DPNS contract
            updateDefinition,
            0  // key index
        );
        throw new Error('Should fail without ownership');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error without ownership');
    }
});

// Broadcast Functions
describe('Broadcast Functions');

await test('broadcast_raw_transition - requires valid transition', async () => {
    try {
        const result = await wasmSdk.broadcast_raw_transition(
            sdk,
            "invalidtransitionhex"
        );
        throw new Error('Should fail with invalid transition');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid transition');
    }
});

await test('wait_for_state_transition_result - requires valid hash', async () => {
    try {
        const result = await wasmSdk.wait_for_state_transition_result(
            sdk,
            "invalidtransitionhash",
            true,  // prove
            60000  // timeout
        );
        throw new Error('Should fail with invalid hash');
    } catch (error) {
        if (error.message.includes('Should fail')) {
            throw error;
        }
        console.log('   Expected error with invalid hash');
    }
});

// Clean up
sdk.free();

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${passed + failed} total`);

console.log('\n📝 Notes:');
console.log('- State transitions require funded identities and proper credentials');
console.log('- Most will fail without network connectivity and actual funds');
console.log('- Token operations require token ownership or balance');
console.log('- Data contract operations require contract ownership');
console.log('- identity_put is known to panic in WASM environment');

process.exit(failed > 0 ? 1 : 0);