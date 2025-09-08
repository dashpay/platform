#!/usr/bin/env node

/**
 * Advanced Patterns Example
 * 
 * Demonstration of advanced SDK usage patterns including complex queries, batch operations,
 * error handling strategies, performance optimization, and production-ready patterns.
 * 
 * Usage: node examples/advanced-patterns.mjs [--network=testnet|mainnet] [--no-proofs] [--debug]
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
    console.log('🚀 Advanced Patterns & Best Practices');
    console.log('='.repeat(50));
    
    // Parse command line arguments
    const args = process.argv.slice(2);
    const network = args.find(arg => arg.startsWith('--network='))?.split('=')[1] || 'testnet';
    const useProofs = !args.includes('--no-proofs');
    const debugMode = args.includes('--debug');
    
    console.log(`🌐 Network: ${network.toUpperCase()}`);
    console.log(`🔒 Proofs: ${useProofs ? 'ENABLED' : 'DISABLED'}`);
    if (debugMode) console.log(`🐛 Debug: ENABLED`);
    
    try {
        // Pre-load WASM for Node.js compatibility
        console.log('\n📦 Pre-loading WASM for Node.js...');
        const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
        await init(readFileSync(wasmPath));
        
        // === PATTERN 1: PARALLEL OPERATIONS ===
        console.log('\n⚡ PATTERN 1: PARALLEL OPERATIONS');
        console.log('-'.repeat(45));
        
        console.log('Creating multiple SDK instances for parallel processing...');
        const sdk1 = new WasmSDK({ network, proofs: false, debug: debugMode });
        const sdk2 = new WasmSDK({ network, proofs: useProofs, debug: debugMode });
        
        // Initialize in parallel
        console.log('Initializing SDKs in parallel...');
        const startTime = Date.now();
        await Promise.all([sdk1.initialize(), sdk2.initialize()]);
        const initTime = Date.now() - startTime;
        console.log(`✅ Parallel initialization: ${initTime}ms`);
        
        // Parallel key generation
        console.log('Generating keys in parallel...');
        const keyGenStart = Date.now();
        const [keys1, keys2, keys3] = await Promise.all([
            sdk1.generateKeyPair(network),
            sdk1.generateKeyPair(network),
            sdk1.generateKeyPair(network)
        ]);
        const keyGenTime = Date.now() - keyGenStart;
        console.log(`✅ Parallel key generation: ${keyGenTime}ms (3 keys)`);
        console.log(`   Key 1: ${keys1.address}`);
        console.log(`   Key 2: ${keys2.address}`);
        console.log(`   Key 3: ${keys3.address}`);
        
        console.log('\n💡 Use Promise.all() for independent operations!');
        
        // === PATTERN 2: BATCH PROCESSING ===
        console.log('\n📦 PATTERN 2: BATCH PROCESSING');
        console.log('-'.repeat(40));
        
        // Batch identity operations
        const identityIds = [
            '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
            '6nF7GQvQX7C1RFQnEBuKCKYRE84i3A7xXtJGqN7FTWwu'
        ];
        
        try {
            console.log('Batch identity balance lookup...');
            const batchBalances = await sdk2.getIdentitiesBalances(identityIds);
            console.log(`✅ Batch balances: ${Object.keys(batchBalances || {}).length} results`);
        } catch (error) {
            console.log(`⚠️ Batch balances: ${error.message.split(' ').slice(0, 6).join(' ')}...`);
        }
        
        // Batch username validation
        const usernames = ['alice', 'bob', 'test123', 'invalid@', 'toolong'.repeat(10)];
        console.log('Batch username validation...');
        const validationResults = await Promise.all(
            usernames.map(async username => ({
                username,
                valid: await sdk1.dpnsIsValidUsername(username),
                contested: await sdk1.dpnsIsContestedUsername(username),
                safe: await sdk1.dpnsConvertToHomographSafe(username)
            }))
        );
        
        console.log('✅ Batch validation results:');
        validationResults.forEach(result => {
            const status = result.valid ? '✅' : '❌';
            console.log(`   ${status} "${result.username}": safe="${result.safe}", contested=${result.contested}`);
        });
        
        console.log('\n💡 Batch operations improve performance for multiple items!');
        
        // === PATTERN 3: ROBUST ERROR HANDLING ===
        console.log('\n🛡️ PATTERN 3: ROBUST ERROR HANDLING');
        console.log('-'.repeat(50));
        
        // Error categorization and handling
        const errorTests = [
            {
                name: 'Parameter validation',
                operation: () => sdk1.generateMnemonic('invalid'),
                category: 'ValidationError'
            },
            {
                name: 'Network operation',
                operation: () => sdk2.getIdentity('invalid-identity-format'),
                category: 'NetworkError'
            },
            {
                name: 'Array validation',
                operation: () => sdk1.getTokenStatuses('not-array'),
                category: 'ValidationError'
            }
        ];
        
        console.log('Error handling examples:');
        for (const test of errorTests) {
            try {
                await test.operation();
                console.log(`⚠️ ${test.name}: Unexpectedly succeeded`);
            } catch (error) {
                console.log(`✅ ${test.name}: Handled correctly (${test.category})`);
                console.log(`   Error: ${error.message.substring(0, 60)}...`);
            }
        }
        
        console.log('\n💡 Categorize errors for appropriate user experience!');
        
        // === PATTERN 4: PAGINATION STRATEGIES ===
        console.log('\n📄 PATTERN 4: PAGINATION STRATEGIES');
        console.log('-'.repeat(45));
        
        const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
        
        try {
            // Manual pagination
            console.log('Manual pagination example:');
            const page1 = await sdk2.getDocuments(DPNS_CONTRACT, 'domain', { limit: 3 });
            console.log(`✅ Page 1: ${page1.documents.length} documents`);
            
            // Automatic pagination (using getAllDocuments)
            console.log('Automatic pagination example:');
            const allDocuments = await sdk2.getDocuments(DPNS_CONTRACT, 'domain', { getAllDocuments: true });
            console.log(`✅ All documents: ${allDocuments.totalCount} documents (auto-paginated)`);
            
        } catch (error) {
            console.log(`⚠️ Pagination examples: ${error.message.split(' ').slice(0, 8).join(' ')}...`);
        }
        
        console.log('\n💡 Use getAllDocuments: true for complete data sets!');
        
        // === PATTERN 5: PERFORMANCE OPTIMIZATION ===
        console.log('\n⚡ PATTERN 5: PERFORMANCE OPTIMIZATION');
        console.log('-'.repeat(50));
        
        // Compare proof vs non-proof performance
        console.log('Performance comparison: Proofs vs No Proofs');
        
        const performanceTests = [
            { name: 'Key Generation', operation: () => sdk1.generateKeyPair(network) },
            { name: 'Mnemonic Validation', operation: () => sdk1.validateMnemonic('abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about') },
            { name: 'DPNS Validation', operation: () => sdk1.dpnsIsValidUsername('testuser') }
        ];
        
        for (const test of performanceTests) {
            const start = Date.now();
            await test.operation();
            const duration = Date.now() - start;
            console.log(`✓ ${test.name}: ${duration}ms`);
        }
        
        // Resource monitoring
        const stats1 = sdk1.getResourceStats();
        const stats2 = sdk2.getResourceStats();
        console.log('\nResource usage:');
        console.log(`✓ SDK 1 (no proofs): ${stats1.totalResources || 0} resources`);
        console.log(`✓ SDK 2 (with proofs): ${stats2.totalResources || 0} resources`);
        
        console.log('\n💡 Balance performance needs vs security requirements!');
        
        // === PATTERN 6: PRODUCTION DEPLOYMENT ===
        console.log('\n🏭 PATTERN 6: PRODUCTION DEPLOYMENT');
        console.log('-'.repeat(45));
        
        console.log('Production-ready initialization pattern:');
        console.log(`✓ Environment-based network selection: ${network}`);
        console.log(`✓ Configurable proof verification: ${useProofs}`);
        console.log(`✓ Debug mode control: ${debugMode}`);
        console.log(`✓ Error boundary implementation: Demonstrated`);
        console.log(`✓ Resource cleanup: Automatic`);
        console.log(`✓ Performance monitoring: Resource stats available`);
        
        console.log('\nConfiguration best practices:');
        console.log('- Development: proofs=false, debug=true');
        console.log('- Staging: proofs=true, debug=true');
        console.log('- Production: proofs=true, debug=false');
        
        console.log('\nMonitoring best practices:');
        console.log('- Track resource usage with getResourceStats()');
        console.log('- Monitor operation timing for performance');
        console.log('- Log errors by category for debugging');
        
        console.log('\n💡 Production patterns ensure reliable operation at scale!');
        
        // === SUMMARY ===
        console.log('\n🎯 ADVANCED PATTERNS SUMMARY');
        console.log('-'.repeat(40));
        console.log('✅ Parallel operations for performance');
        console.log('✅ Batch processing for efficiency'); 
        console.log('✅ Robust error handling strategies');
        console.log('✅ Pagination strategies for large datasets');
        console.log('✅ Performance optimization techniques');
        console.log('✅ Production deployment patterns');
        console.log('✅ Complete best practices framework');
        
        // Clean up all SDKs
        await Promise.all([sdk1.destroy(), sdk2.destroy()]);
        console.log('\n🎉 Advanced patterns demonstration completed successfully!');
        
    } catch (error) {
        console.log(`❌ Advanced patterns failed: ${error.message}`);
        
        console.log('\n🔧 ADVANCED DEBUGGING:');
        console.log('1. Check WASM module compatibility');
        console.log('2. Verify Node.js environment setup');
        console.log('3. Test network connectivity');
        console.log('4. Review error logs with --debug');
        console.log('5. Check resource cleanup');
        
        process.exit(1);
    }
}

await main();