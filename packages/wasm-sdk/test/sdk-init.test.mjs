#!/usr/bin/env node
// sdk-init.test.mjs - Tests for SDK initialization and configuration

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
import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

// Test utilities
const tests = [];
let currentSuite = '';

function describe(name, fn) {
    currentSuite = name;
    console.log(`\n${name}`);
    fn();
}

function test(name, fn) {
    tests.push({ suite: currentSuite, name, fn });
}

function expect(value) {
    return {
        toBe(expected) {
            if (value !== expected) {
                throw new Error(`Expected ${value} to be ${expected}`);
            }
        },
        toBeDefined() {
            if (value === undefined) {
                throw new Error(`Expected value to be defined`);
            }
        },
        toBeInstanceOf(expectedClass) {
            if (!(value instanceof expectedClass)) {
                throw new Error(`Expected value to be instance of ${expectedClass.name}`);
            }
        },
        toContain(substring) {
            if (!value.includes(substring)) {
                throw new Error(`Expected ${value} to contain ${substring}`);
            }
        },
        toThrow() {
            try {
                value();
                throw new Error('Expected function to throw');
            } catch (e) {
                // Expected
            }
        }
    };
}

// Initialize WASM
console.log('Initializing WASM SDK...');
const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
const wasmBuffer = readFileSync(wasmPath);
await init(wasmBuffer);

// Define tests
describe('SDK Initialization Tests', () => {
    describe('WasmSdkBuilder', () => {
        test('should create mainnet SDK', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_mainnet();
            expect(builder).toBeDefined();
            expect(builder).toBeInstanceOf(wasmSdk.WasmSdkBuilder);
            
            const sdk = await builder.build();
            expect(sdk).toBeDefined();
            expect(sdk).toBeInstanceOf(wasmSdk.WasmSdk);
            
            // Free resources
            sdk.free();
            builder.free();
        });
        
        test('should create mainnet trusted SDK', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_mainnet_trusted();
            expect(builder).toBeDefined();
            expect(builder).toBeInstanceOf(wasmSdk.WasmSdkBuilder);
            
            const sdk = await builder.build();
            expect(sdk).toBeDefined();
            expect(sdk).toBeInstanceOf(wasmSdk.WasmSdk);
            
            sdk.free();
            builder.free();
        });
        
        test('should create testnet SDK', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_testnet();
            expect(builder).toBeDefined();
            expect(builder).toBeInstanceOf(wasmSdk.WasmSdkBuilder);
            
            const sdk = await builder.build();
            expect(sdk).toBeDefined();
            expect(sdk).toBeInstanceOf(wasmSdk.WasmSdk);
            
            sdk.free();
            builder.free();
        });
        
        test('should create testnet trusted SDK', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
            expect(builder).toBeDefined();
            expect(builder).toBeInstanceOf(wasmSdk.WasmSdkBuilder);
            
            const sdk = await builder.build();
            expect(sdk).toBeDefined();
            expect(sdk).toBeInstanceOf(wasmSdk.WasmSdk);
            
            sdk.free();
            builder.free();
        });
        
        test('should set custom settings', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_testnet();
            
            // Test with custom settings
            const settings = {
                request_timeout_seconds: 10,
                connect_timeout_seconds: 5,
                retries: 3
            };
            
            builder.with_settings(JSON.stringify(settings));
            
            const sdk = await builder.build();
            expect(sdk).toBeDefined();
            
            sdk.free();
            builder.free();
        });
        
        test('should set specific version', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_testnet();
            
            // Test with version 1
            builder.with_version(1);
            
            const sdk = await builder.build();
            expect(sdk).toBeDefined();
            
            sdk.free();
            builder.free();
        });
        
        test('should handle invalid settings gracefully', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_testnet();
            
            expect(() => {
                builder.with_settings('invalid json');
            }).toThrow();
            
            builder.free();
        });
    });
    
    describe('getLatestVersionNumber', () => {
        test('should return latest version number', () => {
            const version = wasmSdk.WasmSdkBuilder.getLatestVersionNumber();
            expect(version).toBeDefined();
            expect(typeof version).toBe('number');
            expect(version).toBe(1);
            console.log(`  Latest version: ${version}`);
        });
    });
    
    describe('SDK Methods Availability', () => {
        test('should have all query functions available as top-level exports', async () => {
            const builder = wasmSdk.WasmSdkBuilder.new_testnet();
            const sdk = await builder.build();
            
            // Identity queries
            expect(typeof wasmSdk.identity_fetch).toBe('function');
            expect(typeof wasmSdk.identity_fetch_unproved).toBe('function');
            expect(typeof wasmSdk.get_identity_keys).toBe('function');
            expect(typeof wasmSdk.get_identity_nonce).toBe('function');
            expect(typeof wasmSdk.get_identity_contract_nonce).toBe('function');
            expect(typeof wasmSdk.get_identity_balance).toBe('function');
            expect(typeof wasmSdk.get_identities_balances).toBe('function');
            expect(typeof wasmSdk.get_identity_balance_and_revision).toBe('function');
            expect(typeof wasmSdk.get_identity_by_public_key_hash).toBe('function');
            expect(typeof wasmSdk.get_identity_by_non_unique_public_key_hash).toBe('function');
            expect(typeof wasmSdk.get_identities_contract_keys).toBe('function');
            expect(typeof wasmSdk.get_identity_token_balances).toBe('function');
            
            // Document queries
            expect(typeof wasmSdk.get_documents).toBe('function');
            expect(typeof wasmSdk.get_document).toBe('function');
            
            // Data contract queries
            expect(typeof wasmSdk.data_contract_fetch).toBe('function');
            expect(typeof wasmSdk.get_data_contract_history).toBe('function');
            expect(typeof wasmSdk.get_data_contracts).toBe('function');
            
            // Token queries
            expect(typeof wasmSdk.get_identities_token_balances).toBe('function');
            expect(typeof wasmSdk.get_identity_token_infos).toBe('function');
            expect(typeof wasmSdk.get_identities_token_infos).toBe('function');
            expect(typeof wasmSdk.get_token_statuses).toBe('function');
            expect(typeof wasmSdk.get_token_direct_purchase_prices).toBe('function');
            expect(typeof wasmSdk.get_token_contract_info).toBe('function');
            expect(typeof wasmSdk.get_token_perpetual_distribution_last_claim).toBe('function');
            expect(typeof wasmSdk.get_token_total_supply).toBe('function');
            
            // Epoch queries
            expect(typeof wasmSdk.get_epochs_info).toBe('function');
            expect(typeof wasmSdk.get_finalized_epoch_infos).toBe('function');
            expect(typeof wasmSdk.get_current_epoch).toBe('function');
            expect(typeof wasmSdk.get_evonodes_proposed_epoch_blocks_by_ids).toBe('function');
            expect(typeof wasmSdk.get_evonodes_proposed_epoch_blocks_by_range).toBe('function');
            
            // Protocol/System queries
            expect(typeof wasmSdk.get_protocol_version_upgrade_state).toBe('function');
            expect(typeof wasmSdk.get_protocol_version_upgrade_vote_status).toBe('function');
            expect(typeof wasmSdk.get_status).toBe('function');
            expect(typeof wasmSdk.get_current_quorums_info).toBe('function');
            expect(typeof wasmSdk.get_total_credits_in_platform).toBe('function');
            expect(typeof wasmSdk.get_prefunded_specialized_balance).toBe('function');
            expect(typeof wasmSdk.get_path_elements).toBe('function');
            
            // Voting/Contested resources
            expect(typeof wasmSdk.get_contested_resources).toBe('function');
            expect(typeof wasmSdk.get_contested_resource_vote_state).toBe('function');
            expect(typeof wasmSdk.get_contested_resource_voters_for_identity).toBe('function');
            expect(typeof wasmSdk.get_contested_resource_identity_votes).toBe('function');
            expect(typeof wasmSdk.get_vote_polls_by_end_date).toBe('function');
            
            // Group queries
            expect(typeof wasmSdk.get_group_info).toBe('function');
            expect(typeof wasmSdk.get_group_infos).toBe('function');
            expect(typeof wasmSdk.get_group_members).toBe('function');
            expect(typeof wasmSdk.get_identity_groups).toBe('function');
            expect(typeof wasmSdk.get_group_actions).toBe('function');
            expect(typeof wasmSdk.get_group_action_signers).toBe('function');
            expect(typeof wasmSdk.get_groups_data_contracts).toBe('function');
            
            // DPNS queries
            expect(typeof wasmSdk.dpns_register_name).toBe('function');
            expect(typeof wasmSdk.dpns_is_name_available).toBe('function');
            expect(typeof wasmSdk.dpns_resolve_name).toBe('function');
            expect(typeof wasmSdk.get_dpns_username_by_name).toBe('function');
            expect(typeof wasmSdk.get_dpns_usernames).toBe('function');
            expect(typeof wasmSdk.get_dpns_username).toBe('function');
            
            // State transitions - check if they're methods on the SDK instance
            expect(typeof sdk.tokenMint).toBe('function');
            expect(typeof sdk.tokenBurn).toBe('function');
            expect(typeof sdk.tokenTransfer).toBe('function');
            expect(typeof sdk.tokenFreeze).toBe('function');
            expect(typeof sdk.tokenUnfreeze).toBe('function');
            expect(typeof sdk.tokenDestroyFrozen).toBe('function');
            expect(typeof sdk.contractCreate).toBe('function');
            expect(typeof sdk.contractUpdate).toBe('function');
            expect(typeof sdk.documentCreate).toBe('function');
            expect(typeof sdk.documentReplace).toBe('function');
            expect(typeof sdk.documentDelete).toBe('function');
            expect(typeof sdk.documentTransfer).toBe('function');
            expect(typeof sdk.documentPurchase).toBe('function');
            expect(typeof sdk.documentSetPrice).toBe('function');
            expect(typeof sdk.identityCreditTransfer).toBe('function');
            expect(typeof sdk.identityCreditWithdrawal).toBe('function');
            expect(typeof sdk.identityUpdate).toBe('function');
            expect(typeof sdk.masternodeVote).toBe('function');
            
            // Verification functions
            expect(typeof wasmSdk.verify_identity_response).toBe('function');
            expect(typeof wasmSdk.verify_data_contract).toBe('function');
            expect(typeof wasmSdk.verify_documents).toBe('function');
            
            // Utility functions
            expect(typeof wasmSdk.wait_for_state_transition_result).toBe('function');
            
            // Key generation functions
            expect(typeof wasmSdk.generate_mnemonic).toBe('function');
            expect(typeof wasmSdk.validate_mnemonic).toBe('function');
            expect(typeof wasmSdk.mnemonic_to_seed).toBe('function');
            expect(typeof wasmSdk.derive_key_from_seed_phrase).toBe('function');
            expect(typeof wasmSdk.derive_key_from_seed_with_path).toBe('function');
            expect(typeof wasmSdk.generate_key_pair).toBe('function');
            expect(typeof wasmSdk.generate_key_pairs).toBe('function');
            expect(typeof wasmSdk.key_pair_from_wif).toBe('function');
            expect(typeof wasmSdk.key_pair_from_hex).toBe('function');
            expect(typeof wasmSdk.pubkey_to_address).toBe('function');
            expect(typeof wasmSdk.validate_address).toBe('function');
            expect(typeof wasmSdk.sign_message).toBe('function');
            
            // DPNS utility functions
            expect(typeof wasmSdk.dpns_convert_to_homograph_safe).toBe('function');
            expect(typeof wasmSdk.dpns_is_valid_username).toBe('function');
            expect(typeof wasmSdk.dpns_is_contested_username).toBe('function');
            
            // Clean up
            if (sdk && sdk.__wbg_ptr) {
                sdk.free();
            }
            if (builder && builder.__wbg_ptr) {
                builder.free();
            }
        });
    });
});

// Run tests
console.log('\nRunning tests...\n');
let passed = 0;
let failed = 0;

for (const { suite, name, fn } of tests) {
    try {
        await fn();
        console.log(`  ✅ ${name}`);
        passed++;
    } catch (error) {
        console.log(`  ❌ ${name}`);
        console.log(`     ${error.message}`);
        failed++;
    }
}

console.log(`\n\nTest Results: ${passed} passed, ${failed} failed, ${tests.length} total`);
process.exit(failed > 0 ? 1 : 0);