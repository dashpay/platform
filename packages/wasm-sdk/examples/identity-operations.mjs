#!/usr/bin/env node

/**
 * Identity Operations Example
 * 
 * Comprehensive demonstration of identity queries, balance operations, and key management.
 * Shows identity lookup, balance queries, key operations, and multi-identity operations.
 * 
 * Usage: node examples/identity-operations.mjs [identity-id] [--network=testnet|mainnet] [--no-proofs] [--debug]
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Set up Node.js environment for WASM
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
if (!global.crypto) global.crypto = webcrypto;

// Import JavaScript wrapper (the correct approach)
import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

async function main() {
    console.log('👤 Comprehensive Identity Operations Example');
    console.log('='.repeat(55));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const identityId = args.find(arg => !arg.startsWith('--')) || '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const useProofs = !args.includes('--no-proofs');
    const debugMode = args.includes('--debug');
    
    console.log(`👤 Identity: ${identityId}`);
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'}`);
    if (debugMode) console.log(`🐛 Debug: ENABLED`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('\n📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        // Initialize JavaScript wrapper
        console.log('📦 Initializing JavaScript wrapper...');
        const sdk = new WasmSDK({
            network: network,
            proofs: useProofs,
            debug: debugMode
        });
        await sdk.initialize();
        console.log('✅ SDK initialized successfully\n');
        
        // === BASIC IDENTITY OPERATIONS ===
        console.log('🆔 BASIC IDENTITY LOOKUP');
        console.log('-'.repeat(35));
        
        try {
            const identity = await sdk.getIdentity(identityId);
            if (identity) {
                console.log(`✅ Identity found:`);
                console.log(`   ID: ${identity.id || identity.$id || 'N/A'}`);
                console.log(`   Revision: ${identity.revision || 'N/A'}`);
                console.log(`   Balance: ${identity.balance || 'N/A'} credits`);
                console.log(`   Public Keys: ${identity.publicKeys?.length || 0} keys`);
            } else {
                console.log(`⚠️ Identity not found or private`);
            }
        } catch (error) {
            console.log(`⚠️ Identity lookup failed: ${error.message}`);
        }
        
        // === BALANCE OPERATIONS ===
        console.log('\n💰 BALANCE OPERATIONS');
        console.log('-'.repeat(25));
        
        try {
            const balance = await sdk.getIdentityBalance(identityId);
            console.log(`✅ Current balance: ${balance} credits`);
            
            const balanceAndRevision = await sdk.getIdentityBalanceAndRevision(identityId);
            console.log(`✅ Balance with revision:`);
            console.log(`   Balance: ${balanceAndRevision.balance || 'N/A'} credits`);
            console.log(`   Revision: ${balanceAndRevision.revision || 'N/A'}`);
        } catch (error) {
            console.log(`⚠️ Balance operations failed: ${error.message}`);
        }
        
        // === KEY OPERATIONS ===
        console.log('\n🔐 KEY OPERATIONS');
        console.log('-'.repeat(20));
        
        try {
            const keys = await sdk.getIdentityKeys(identityId);
            if (keys && keys.length > 0) {
                console.log(`✅ Found ${keys.length} keys:`);
                keys.forEach((key, index) => {
                    console.log(`   Key ${index + 1}:`);
                    console.log(`     ID: ${key.id}`);
                    console.log(`     Type: ${key.type}`);
                    console.log(`     Purpose: ${key.purpose}`);
                    console.log(`     Security Level: ${key.securityLevel}`);
                });
            } else {
                console.log(`⚠️ No keys found or private`);
            }
            
            const nonce = await sdk.getIdentityNonce(identityId);
            console.log(`✅ Identity nonce: ${nonce}`);
        } catch (error) {
            console.log(`⚠️ Key operations failed: ${error.message}`);
        }
        
        // === MULTI-IDENTITY OPERATIONS ===
        console.log('\n👥 MULTI-IDENTITY OPERATIONS');
        console.log('-'.repeat(35));
        
        const identityIds = [identityId, '6nF7GQvQX7C1RFQnEBuKCKYRE84i3A7xXtJGqN7FTWwu']; // Add another test identity
        
        try {
            const balances = await sdk.getIdentitiesBalances(identityIds);
            console.log(`✅ Multiple identity balances:`);
            Object.entries(balances || {}).forEach(([id, balance]) => {
                console.log(`   ${id}: ${balance} credits`);
            });
        } catch (error) {
            console.log(`⚠️ Multi-identity balances failed: ${error.message}`);
        }
        
        // === TOKEN-RELATED IDENTITY OPERATIONS ===
        console.log('\n🪙 TOKEN-RELATED OPERATIONS');
        console.log('-'.repeat(35));
        
        try {
            const tokenIds = ['example-token-1', 'example-token-2'];
            const tokenBalances = await sdk.getIdentityTokenBalances(identityId, tokenIds);
            console.log(`✅ Token balances for identity:`);
            console.log(`   Queried tokens: ${tokenIds.length}`);
            console.log(`   Results: ${Object.keys(tokenBalances || {}).length} token balances`);
            
            const tokenInfos = await sdk.getIdentityTokenInfos(identityId, tokenIds, 10, 0);
            console.log(`✅ Token information retrieved`);
        } catch (error) {
            console.log(`⚠️ Token operations failed: ${error.message} (expected without real tokens)`);
        }
        
        // === PUBLIC KEY HASH OPERATIONS ===
        console.log('\n🔍 PUBLIC KEY HASH OPERATIONS');
        console.log('-'.repeat(40));
        
        // Generate a test key to demonstrate public key hash operations
        const testKey = await sdk.generateKeyPair('testnet');
        const testHash = "1234567890abcdef1234567890abcdef12345678"; // Example 40-char hash
        
        try {
            const identityByHash = await sdk.getIdentityByPublicKeyHash(testHash);
            console.log(`✅ Identity lookup by unique hash completed`);
            
            const identitiesByHash = await sdk.getIdentityByNonUniquePublicKeyHash(testHash);
            console.log(`✅ Identity lookup by non-unique hash completed`);
        } catch (error) {
            console.log(`⚠️ Public key hash operations: ${error.message} (expected without matching hashes)`);
        }
        
        // === PRACTICAL EXAMPLES ===
        console.log('\n🛠️  PRACTICAL EXAMPLES');
        console.log('-'.repeat(25));
        
        // Example: Create a new wallet
        console.log('Example 1: Create New Wallet');
        const newMnemonic = await sdk.generateMnemonic(12);
        const newSeed = await sdk.mnemonicToSeed(newMnemonic);
        const newKey = await sdk.deriveKeyFromSeedWithPath(newMnemonic, '', "m/44'/1'/0'/0/0", 'testnet');
        console.log(`✓ New wallet address: ${newKey.address}`);
        
        // Example: Validate user input
        console.log('\nExample 2: Validate User Input');
        const userInputs = [
            'valid mnemonic here abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about',
            'invalid mnemonic input',
            newKey.address,
            'invalid_address'
        ];
        
        for (let i = 0; i < userInputs.length; i += 2) {
            const mnemonicValid = await sdk.validateMnemonic(userInputs[i]);
            const addressValid = await sdk.validateAddress(userInputs[i + 1], 'testnet');
            console.log(`✓ Mnemonic validation: ${mnemonicValid}, Address validation: ${addressValid}`);
        }
        
        // Example: Message signing workflow
        console.log('\nExample 3: Message Signing Workflow');
        const message = "I authorize this transaction";
        const signature = await sdk.signMessage(message, newKey.private_key_wif);
        console.log(`✓ Message: "${message}"`);
        console.log(`✓ Signature: ${signature.substring(0, 40)}...`);
        
        // === SUMMARY ===
        console.log('\n📊 OPERATION SUMMARY');
        console.log('-'.repeat(25));
        console.log('✅ Identity lookup operations');
        console.log('✅ Balance and revision queries');
        console.log('✅ Key management operations');
        console.log('✅ Multi-identity operations');
        console.log('✅ Token-related queries');
        console.log('✅ Public key hash operations');
        console.log('✅ Practical wallet examples');
        console.log('✅ Message signing workflows');
        
        // Clean up
        await sdk.destroy();
        console.log('\n🎉 Identity operations demonstration completed successfully!');
        
    } catch (error) {
        console.log(`❌ Identity operations failed: ${error.message}`);
        
        if (error.message.includes('fetch') || error.message.includes('network')) {
            console.log('🌐 Network connectivity issue - some operations require online connection');
        } else if (error.message.includes('not found')) {
            console.log('👤 Identity may not exist on this network');
        }
        
        console.log('\nFor debugging:');
        console.log('1. Try with different identity ID');
        console.log('2. Check network connectivity');
        console.log('3. Use --no-proofs for faster testing');
        console.log('4. Try --debug for detailed output');
        
        process.exit(1);
    }
}

await main();