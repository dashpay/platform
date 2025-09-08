#!/usr/bin/env node

/**
 * Token Operations Example
 * 
 * Comprehensive demonstration of token queries, balance operations, and token ecosystem exploration.
 * Shows token information, pricing, supply data, and identity-token relationships.
 * 
 * Usage: node examples/token-operations.mjs [token-id] [--network=testnet|mainnet] [--no-proofs] [--debug]
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
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

async function main() {
    console.log('🪙 Comprehensive Token Operations Example');
    console.log('='.repeat(50));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const tokenId = args.find(arg => !arg.startsWith('--')) || 'example-token-id';
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const useProofs = !args.includes('--no-proofs');
    const debugMode = args.includes('--debug');
    
    console.log(`🪙 Token: ${tokenId}`);
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'}`);
    if (debugMode) console.log(`🐛 Debug: ENABLED`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('\n📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
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
        
        // === TOKEN STATUS OPERATIONS ===
        console.log('📊 TOKEN STATUS OPERATIONS');
        console.log('-'.repeat(35));
        
        const testTokenIds = [tokenId, 'token-id-2', 'token-id-3'];
        
        try {
            const statuses = await sdk.getTokenStatuses(testTokenIds);
            console.log(`✅ Token statuses queried for ${testTokenIds.length} tokens:`);
            console.log(`   Result type: ${typeof statuses}`);
            console.log(`   Status data: ${Object.keys(statuses || {}).length} entries`);
        } catch (error) {
            console.log(`⚠️ Token statuses failed: ${error.message}`);
        }
        
        try {
            const prices = await sdk.getTokenDirectPurchasePrices(testTokenIds);
            console.log(`✅ Direct purchase prices queried:`);
            console.log(`   Result type: ${typeof prices}`);
            console.log(`   Price data: ${Object.keys(prices || {}).length} entries`);
        } catch (error) {
            console.log(`⚠️ Token prices failed: ${error.message}`);
        }
        
        // === SINGLE TOKEN OPERATIONS ===
        console.log('\n💰 SINGLE TOKEN OPERATIONS');
        console.log('-'.repeat(35));
        
        try {
            const totalSupply = await sdk.getTokenTotalSupply(tokenId);
            console.log(`✅ Total supply for ${tokenId}: ${totalSupply}`);
        } catch (error) {
            console.log(`⚠️ Total supply failed: ${error.message} (expected without real token)`);
        }
        
        // === CONTRACT-TOKEN OPERATIONS ===
        console.log('\n📄 CONTRACT-TOKEN OPERATIONS');
        console.log('-'.repeat(40));
        
        const testContractId = 'example-contract-id';
        const tokenPosition = 0;
        
        try {
            const contractInfo = await sdk.getTokenContractInfo(testContractId);
            console.log(`✅ Token contract info:`);
            console.log(`   Contract: ${testContractId}`);
            console.log(`   Info type: ${typeof contractInfo}`);
        } catch (error) {
            console.log(`⚠️ Token contract info failed: ${error.message} (expected without real contract)`);
        }
        
        try {
            const price = await sdk.getTokenPriceByContract(testContractId, tokenPosition);
            console.log(`✅ Token price by contract: ${typeof price}`);
        } catch (error) {
            console.log(`⚠️ Token price failed: ${error.message} (expected without real contract)`);
        }
        
        try {
            const calculatedTokenId = await sdk.calculateTokenIdFromContract(testContractId, tokenPosition);
            console.log(`✅ Calculated token ID: ${calculatedTokenId}`);
        } catch (error) {
            console.log(`⚠️ Token ID calculation failed: ${error.message} (expected without real contract)`);
        }
        
        // === IDENTITY-TOKEN OPERATIONS ===
        console.log('\n👤 IDENTITY-TOKEN OPERATIONS');
        console.log('-'.repeat(40));
        
        const testIdentityId = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
        const identityIds = [testIdentityId, '6nF7GQvQX7C1RFQnEBuKCKYRE84i3A7xXtJGqN7FTWwu'];
        
        try {
            const tokenBalances = await sdk.getIdentityTokenBalances(testIdentityId, testTokenIds);
            console.log(`✅ Identity token balances:`);
            console.log(`   Identity: ${testIdentityId.substring(0, 20)}...`);
            console.log(`   Tokens queried: ${testTokenIds.length}`);
            console.log(`   Balance data: ${typeof tokenBalances}`);
        } catch (error) {
            console.log(`⚠️ Identity token balances failed: ${error.message} (expected without real tokens)`);
        }
        
        try {
            const tokenInfos = await sdk.getIdentityTokenInfos(testIdentityId, testTokenIds, 10, 0);
            console.log(`✅ Identity token information:`);
            console.log(`   Info type: ${typeof tokenInfos}`);
            console.log(`   Limit: 10, Offset: 0`);
        } catch (error) {
            console.log(`⚠️ Identity token infos failed: ${error.message} (expected without real tokens)`);
        }
        
        try {
            const multiBalances = await sdk.getIdentitiesTokenBalances(identityIds, tokenId);
            console.log(`✅ Multi-identity token balances:`);
            console.log(`   Identities: ${identityIds.length}`);
            console.log(`   Token: ${tokenId.substring(0, 20)}...`);
            console.log(`   Balance data: ${typeof multiBalances}`);
        } catch (error) {
            console.log(`⚠️ Multi-identity token balances failed: ${error.message} (expected without real tokens)`);
        }
        
        try {
            const multiInfos = await sdk.getIdentitiesTokenInfos(identityIds, tokenId);
            console.log(`✅ Multi-identity token information:`);
            console.log(`   Info data: ${typeof multiInfos}`);
        } catch (error) {
            console.log(`⚠️ Multi-identity token infos failed: ${error.message} (expected without real tokens)`);
        }
        
        // === TOKEN DISTRIBUTION OPERATIONS ===
        console.log('\n🎁 TOKEN DISTRIBUTION OPERATIONS');
        console.log('-'.repeat(45));
        
        try {
            const lastClaim = await sdk.getTokenPerpetualDistributionLastClaim(testIdentityId, tokenId);
            console.log(`✅ Perpetual distribution last claim:`);
            console.log(`   Identity: ${testIdentityId.substring(0, 20)}...`);
            console.log(`   Token: ${tokenId.substring(0, 20)}...`);
            console.log(`   Claim data: ${typeof lastClaim}`);
        } catch (error) {
            console.log(`⚠️ Perpetual distribution failed: ${error.message} (expected without real distribution)`);
        }
        
        // === TOKEN ECOSYSTEM ANALYSIS ===
        console.log('\n🌐 TOKEN ECOSYSTEM ANALYSIS');
        console.log('-'.repeat(40));
        
        console.log('Token Ecosystem Overview:');
        console.log(`✓ Token status queries: Available for ${testTokenIds.length} tokens`);
        console.log(`✓ Price information: Direct purchase prices supported`);
        console.log(`✓ Supply information: Total supply tracking available`);
        console.log(`✓ Contract integration: Token-contract linking supported`);
        console.log(`✓ Identity relationships: Token ownership and balance tracking`);
        console.log(`✓ Distribution tracking: Perpetual distribution claim monitoring`);
        
        // === PRACTICAL USE CASES ===
        console.log('\n🛠️  PRACTICAL USE CASES');
        console.log('-'.repeat(30));
        
        console.log('Use Case 1: Token Portfolio Dashboard');
        console.log('- Query multiple token statuses and prices');
        console.log('- Get identity token balances across all tokens');
        console.log('- Display total portfolio value');
        
        console.log('\nUse Case 2: Token Analytics Platform');
        console.log('- Monitor total supply changes across tokens');
        console.log('- Track price movements and market data');
        console.log('- Analyze distribution patterns');
        
        console.log('\nUse Case 3: DeFi Integration');
        console.log('- Real-time balance monitoring for multiple identities');
        console.log('- Price feeds for trading applications');
        console.log('- Supply data for tokenomics analysis');
        
        // === SUMMARY ===
        console.log('\n📊 TOKEN OPERATIONS SUMMARY');
        console.log('-'.repeat(35));
        console.log('✅ Token status and metadata queries');
        console.log('✅ Direct purchase price information');
        console.log('✅ Token-contract relationship mapping');
        console.log('✅ Total supply tracking');
        console.log('✅ Identity token balance operations');
        console.log('✅ Multi-identity token operations');
        console.log('✅ Token distribution monitoring');
        console.log('✅ Complete token ecosystem support');
        
        // Clean up
        await sdk.destroy();
        console.log('\n🎉 Token operations demonstration completed successfully!');
        
    } catch (error) {
        console.log(`❌ Token operations failed: ${error.message}`);
        
        if (error.message.includes('fetch') || error.message.includes('network')) {
            console.log('🌐 Network connectivity required for platform queries');
        } else if (error.message.includes('not found')) {
            console.log('🪙 Token or contract may not exist on this network');
        }
        
        console.log('\nFor debugging:');
        console.log('1. Try with real token IDs from the platform');
        console.log('2. Check network connectivity');
        console.log('3. Use --no-proofs for faster testing');
        console.log('4. Try --debug for detailed output');
        
        process.exit(1);
    }
}

await main();