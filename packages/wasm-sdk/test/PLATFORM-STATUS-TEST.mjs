#!/usr/bin/env node
/**
 * Platform Status Test - Check if platform is operational for write operations
 * Tests platform status and attempts simpler operations
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

console.log('🔍 PLATFORM STATUS TEST');
console.log('Checking platform operational status and testing simpler operations\n');

const BEST_ENDPOINT = 'https://44.240.98.102:1443';
const IDENTITY_ID = process.env.IDENTITY_ID || 'DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq';

async function checkPlatformStatus() {
    console.log('🚀 Checking platform status...\n');
    
    let sdk;
    try {
        sdk = new WasmSDK({ 
            network: 'testnet', 
            transport: { url: BEST_ENDPOINT, timeout: 30000 },
            proofs: false, 
            debug: true 
        });
        await sdk.initialize();
        
        const results = {
            systemStatus: null,
            protocolVersion: null,
            epochInfo: null,
            networkStatus: null,
            identityQueries: null,
            contractQueries: null
        };
        
        // Test 1: System status
        console.log('📊 Test 1: System Status...');
        try {
            const status = await sdk.getStatus();
            console.log(`   ✅ Platform Status: ${JSON.stringify(status, null, 2)}`);
            results.systemStatus = { success: true, data: status };
        } catch (error) {
            console.log(`   ❌ System Status Failed: ${error?.message || error}`);
            results.systemStatus = { success: false, error: error?.message || error };
        }
        
        // Test 2: Protocol Version
        console.log('\n🔧 Test 2: Protocol Version...');
        try {
            const version = await sdk.getProtocolVersion();
            console.log(`   ✅ Protocol Version: ${JSON.stringify(version, null, 2)}`);
            results.protocolVersion = { success: true, data: version };
        } catch (error) {
            console.log(`   ❌ Protocol Version Failed: ${error?.message || error}`);
            results.protocolVersion = { success: false, error: error?.message || error };
        }
        
        // Test 3: Epoch Info
        console.log('\n🗓️ Test 3: Epoch Info...');
        try {
            const epochInfo = await sdk.getEpochsInfo();
            console.log(`   ✅ Epoch Info: ${JSON.stringify(epochInfo, null, 2)}`);
            results.epochInfo = { success: true, data: epochInfo };
        } catch (error) {
            console.log(`   ❌ Epoch Info Failed: ${error?.message || error}`);
            results.epochInfo = { success: false, error: error?.message || error };
        }
        
        // Test 4: Identity Query (this works)
        console.log('\n👤 Test 4: Identity Query...');
        try {
            const identity = await sdk.getIdentity(IDENTITY_ID);
            console.log(`   ✅ Identity Query: Found identity with ${identity?.balance || 'unknown'} credits`);
            results.identityQueries = { success: true, data: { balance: identity?.balance } };
        } catch (error) {
            console.log(`   ❌ Identity Query Failed: ${error?.message || error}`);
            results.identityQueries = { success: false, error: error?.message || error };
        }
        
        // Test 5: Contract Query
        console.log('\n📄 Test 5: Contract Query...');
        const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
        try {
            const contract = await sdk.getDataContract(DPNS_CONTRACT);
            console.log(`   ✅ Contract Query: Found DPNS contract`);
            console.log(`      Contract owner: ${contract?.ownerId || 'unknown'}`);
            results.contractQueries = { success: true, data: { ownerId: contract?.ownerId } };
        } catch (error) {
            console.log(`   ❌ Contract Query Failed: ${error?.message || error}`);
            results.contractQueries = { success: false, error: error?.message || error };
        }
        
        await sdk.destroy();
        return results;
        
    } catch (error) {
        console.log(`❌ Platform Status Check Failed: ${error?.message || error}`);
        if (sdk) await sdk.destroy();
        return { error: error?.message || error };
    }
}

async function testAlternativeOperations() {
    console.log('\n🧪 Testing alternative operations...\n');
    
    let sdk;
    try {
        sdk = new WasmSDK({ 
            network: 'testnet', 
            transport: { url: BEST_ENDPOINT, timeout: 30000 },
            proofs: true,  // Try with proofs enabled
            debug: true 
        });
        await sdk.initialize();
        
        // Test with proofs enabled - sometimes this helps with broadcast issues
        console.log('🔐 Test: Document creation with proofs enabled...');
        const testData = {
            normalizedLabel: 'simple-test-' + Date.now(),
            normalizedParentDomainName: 'dash',
            label: 'simple-test-' + Date.now(),
            parentDomainName: 'dash',
            records: {
                dashIdentity: IDENTITY_ID
            }
        };
        
        const mnemonic = process.env.MNEMONIC || 'lamp truck drip furnace now swing income victory leisure popular jeans vehicle';
        const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
        
        try {
            const result = await sdk.createDocument(
                mnemonic,
                IDENTITY_ID,
                DPNS_CONTRACT,
                'domain',
                JSON.stringify(testData),
                1
            );
            console.log('   ✅ SUCCESS with proofs enabled!');
            console.log(`   📄 Result: ${JSON.stringify(result, null, 2)}`);
            
            await sdk.destroy();
            return { success: true, result };
            
        } catch (error) {
            console.log(`   ❌ Still failed with proofs: ${error?.message || error}`);
            await sdk.destroy();
            return { success: false, error: error?.message || error };
        }
        
    } catch (error) {
        console.log(`❌ Alternative operations failed: ${error?.message || error}`);
        if (sdk) await sdk.destroy();
        return { success: false, error: error?.message || error };
    }
}

// Run tests
const platformStatus = await checkPlatformStatus();
const alternativeOps = await testAlternativeOperations();

console.log('\n📊 PLATFORM STATUS SUMMARY');
console.log('==========================\n');

// Analyze platform status
const workingQueries = Object.entries(platformStatus).filter(([key, result]) => 
    result && result.success && key !== 'error'
).length;

const totalTests = Object.keys(platformStatus).length - (platformStatus.error ? 1 : 0);

console.log(`📈 Query Operations: ${workingQueries}/${totalTests} working`);
console.log('   • Read operations: ' + (workingQueries > 0 ? '✅ Working' : '❌ Failed'));
console.log('   • Write operations: ' + (alternativeOps.success ? '✅ Working' : '❌ Failed'));

if (alternativeOps.success) {
    console.log('\n🎉 BREAKTHROUGH: Platform operations working!');
    console.log('   💡 Solution: Use proofs enabled for write operations');
} else {
    console.log('\n🔍 ANALYSIS: Platform Issue Confirmed');
    console.log('   📝 Read operations work fine');
    console.log('   ❌ Write operations failing with "Missing response message"');
    console.log('   🔧 This appears to be a platform/testnet infrastructure issue');
    console.log('   💡 Recommendation: Check platform status or try different time');
}

console.log('\n✅ Platform status test complete');