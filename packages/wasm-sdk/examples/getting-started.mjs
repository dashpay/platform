#!/usr/bin/env node

/**
 * Getting Started Tutorial
 * 
 * Complete beginner-friendly tutorial showing basic WASM SDK usage.
 * Covers initialization, basic queries, key operations, and common patterns.
 * 
 * Usage: node examples/getting-started.mjs [--network=testnet|mainnet] [--debug]
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
    console.log('🚀 Getting Started with Dash Platform WASM SDK');
    console.log('='.repeat(60));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const debugMode = args.includes('--debug');
    
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    if (debugMode) console.log(`🐛 Debug: ENABLED`);
    
    try {
        // === STEP 1: SETUP AND INITIALIZATION ===
        console.log('\n📚 STEP 1: Setup and Initialization');
        console.log('-'.repeat(45));
        
        console.log('1. Pre-loading WASM module...');
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        console.log('   ✅ WASM module loaded');
        
        console.log('2. Creating SDK instance...');
        const sdk = new WasmSDK({
            network: network,
            proofs: false,  // Start with proofs disabled for speed
            debug: debugMode
        });
        console.log('   ✅ SDK instance created');
        
        console.log('3. Initializing SDK...');
        await sdk.initialize();
        console.log('   ✅ SDK initialized successfully');
        
        console.log('\n💡 Key Learning: Always initialize before using any SDK methods!');
        
        // === STEP 2: BASIC CRYPTOGRAPHIC OPERATIONS ===
        console.log('\n🔐 STEP 2: Basic Cryptographic Operations');
        console.log('-'.repeat(50));
        
        console.log('1. Generating a mnemonic phrase...');
        const mnemonic = await sdk.generateMnemonic(12);
        console.log(`   ✅ Generated: ${mnemonic.split(' ').slice(0, 3).join(' ')}... (12 words)`);
        
        console.log('2. Validating the mnemonic...');
        const isValid = await sdk.validateMnemonic(mnemonic);
        console.log(`   ✅ Validation result: ${isValid}`);
        
        console.log('3. Generating a key pair...');
        const keyPair = await sdk.generateKeyPair(network);
        console.log(`   ✅ Address: ${keyPair.address}`);
        console.log(`   ✅ Public Key: ${keyPair.public_key.substring(0, 20)}...`);
        
        console.log('4. Signing a message...');
        const message = "Hello, Dash Platform!";
        const signature = await sdk.signMessage(message, keyPair.private_key_wif);
        console.log(`   ✅ Message: "${message}"`);
        console.log(`   ✅ Signature: ${signature.substring(0, 30)}...`);
        
        console.log('\n💡 Key Learning: All crypto operations work offline and are deterministic!');
        
        // === STEP 3: PLATFORM QUERIES ===
        console.log('\n🌐 STEP 3: Platform Queries');
        console.log('-'.repeat(35));
        
        console.log('1. Checking platform status...');
        try {
            const status = await sdk.getStatus();
            console.log('   ✅ Platform is accessible');
            console.log(`   📊 Latest block: ${status.chain?.latestBlockHeight || 'N/A'}`);
        } catch (error) {
            console.log(`   ⚠️ Platform query failed: ${error.message.split(' ').slice(0, 5).join(' ')}...`);
            console.log('   💡 Tip: Network connection required for platform queries');
        }
        
        console.log('2. Looking up a known identity...');
        const knownIdentity = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
        try {
            const identity = await sdk.getIdentity(knownIdentity);
            if (identity) {
                console.log('   ✅ Identity found');
                console.log(`   👤 ID: ${identity.id || identity.$id}`);
                console.log(`   💰 Balance: ${identity.balance || 'N/A'} credits`);
            } else {
                console.log('   ⚠️ Identity not found or private');
            }
        } catch (error) {
            console.log(`   ⚠️ Identity lookup failed: ${error.message.split(' ').slice(0, 5).join(' ')}...`);
        }
        
        console.log('3. Exploring DPNS domains...');
        const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
        try {
            const domainsResponse = await sdk.getDocuments(DPNS_CONTRACT, 'domain', { limit: 3 });
            console.log('   ✅ DPNS domains accessed');
            console.log(`   📊 Total domains: ${domainsResponse.totalCount}`);
            console.log(`   📝 Sample domains: ${domainsResponse.documents.length} retrieved`);
        } catch (error) {
            console.log(`   ⚠️ Domain query failed: ${error.message.split(' ').slice(0, 5).join(' ')}...`);
        }
        
        console.log('\n💡 Key Learning: Platform queries require network connection!');
        
        // === STEP 4: PRACTICAL EXAMPLES ===
        console.log('\n🛠️ STEP 4: Practical Examples');
        console.log('-'.repeat(35));
        
        console.log('Example A: Create a new wallet');
        const newMnemonic = await sdk.generateMnemonic(12);
        const newKey = await sdk.deriveKeyFromSeedWithPath(newMnemonic, '', "m/44'/1'/0'/0/0", network);
        console.log(`   ✅ New wallet address: ${newKey.address}`);
        
        console.log('Example B: Validate user input');
        const userMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        const userMnemonicValid = await sdk.validateMnemonic(userMnemonic);
        const userAddressValid = await sdk.validateAddress(newKey.address, network);
        console.log(`   ✅ User mnemonic valid: ${userMnemonicValid}`);
        console.log(`   ✅ User address valid: ${userAddressValid}`);
        
        console.log('Example C: Username validation');
        const proposedUsername = 'myusername';
        const usernameValid = await sdk.dpnsIsValidUsername(proposedUsername);
        const usernameContested = await sdk.dpnsIsContestedUsername(proposedUsername);
        console.log(`   ✅ Username "${proposedUsername}" valid: ${usernameValid}`);
        console.log(`   ✅ Username contested: ${usernameContested}`);
        
        // === STEP 5: ERROR HANDLING ===
        console.log('\n⚠️ STEP 5: Error Handling Patterns');
        console.log('-'.repeat(45));
        
        console.log('Demonstrating proper error handling:');
        
        // Example 1: Parameter validation errors
        try {
            await sdk.generateMnemonic(13); // Invalid word count
        } catch (error) {
            console.log(`✅ Parameter validation: ${error.message.substring(0, 50)}...`);
        }
        
        // Example 2: Network errors
        try {
            await sdk.getIdentity('invalid-identity-id');
        } catch (error) {
            console.log(`✅ Network error handling: ${error.message.split(' ').slice(0, 8).join(' ')}...`);
        }
        
        // Example 3: Array validation errors
        try {
            await sdk.getTokenStatuses('not-an-array');
        } catch (error) {
            console.log(`✅ Array validation: ${error.message.substring(0, 50)}...`);
        }
        
        console.log('\n💡 Key Learning: Always wrap SDK calls in try-catch blocks!');
        
        // === STEP 6: RESOURCE MANAGEMENT ===
        console.log('\n🧹 STEP 6: Resource Management');
        console.log('-'.repeat(35));
        
        console.log('Checking resource usage...');
        const stats = sdk.getResourceStats();
        console.log(`✅ Resource statistics: ${stats.totalResources || 0} resources managed`);
        console.log(`✅ Memory safety: Automatic cleanup enabled`);
        
        console.log('Performing cleanup...');
        await sdk.destroy();
        console.log('✅ SDK destroyed and resources cleaned up');
        
        console.log('\n💡 Key Learning: Always call destroy() when finished!');
        
        // === TUTORIAL COMPLETION ===
        console.log('\n🎓 TUTORIAL COMPLETE!');
        console.log('-'.repeat(25));
        console.log('✅ SDK setup and initialization');
        console.log('✅ Cryptographic operations');
        console.log('✅ Platform queries');
        console.log('✅ Practical examples');
        console.log('✅ Error handling');
        console.log('✅ Resource management');
        
        console.log('\n📖 NEXT STEPS:');
        console.log('• Explore other examples: identity-operations.mjs, dpns-management.mjs');
        console.log('• Try the web interface: open index.html');
        console.log('• Read the documentation: AI_REFERENCE.md');
        console.log('• Check advanced patterns: advanced-patterns.mjs');
        
        console.log('\n🎉 Getting started tutorial completed successfully!');
        
    } catch (error) {
        console.log(`❌ Tutorial failed: ${error.message}`);
        
        console.log('\n🆘 TROUBLESHOOTING:');
        console.log('1. Make sure WASM module is built: ./build.sh');
        console.log('2. Check Node.js version compatibility');
        console.log('3. Try with --debug for more information');
        console.log('4. Verify network connectivity for platform queries');
        
        process.exit(1);
    }
}

await main();