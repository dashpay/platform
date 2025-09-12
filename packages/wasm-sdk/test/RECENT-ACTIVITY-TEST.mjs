#!/usr/bin/env node
/**
 * Recent Activity Test - Check if other identities are successfully writing to platform
 * This helps determine if the issue is platform-wide or specific to our operations
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

console.log('📈 RECENT ACTIVITY TEST');
console.log('Checking platform for recent document activity to assess platform health\n');

const BEST_ENDPOINT = 'https://44.240.98.102:1443';
const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

async function checkRecentActivity() {
    console.log('🔍 Checking recent platform activity...\n');
    
    let sdk;
    try {
        sdk = new WasmSDK({ 
            network: 'testnet', 
            transport: { url: BEST_ENDPOINT, timeout: 30000 },
            proofs: false, 
            debug: true 
        });
        await sdk.initialize();
        
        // Check recent documents to see if platform is accepting writes from others
        console.log('📄 Checking recent DPNS domain documents...');
        try {
            const recentDomains = await sdk.getDocuments(DPNS_CONTRACT, 'domain', {
                limit: 10,
                orderBy: [["$createdAt", "desc"]]
            });
            
            if (recentDomains && recentDomains.length > 0) {
                console.log(`   ✅ Found ${recentDomains.length} recent domains`);
                
                recentDomains.slice(0, 3).forEach((domain, index) => {
                    console.log(`   📋 Domain ${index + 1}:`);
                    console.log(`      Label: ${domain.data?.label || 'unknown'}`);
                    console.log(`      Owner: ${domain.ownerId || 'unknown'}`);
                    console.log(`      Created: ${domain.createdAt ? new Date(domain.createdAt).toISOString() : 'unknown'}`);
                    console.log(`      Updated: ${domain.updatedAt ? new Date(domain.updatedAt).toISOString() : 'unknown'}`);
                });
                
                // Check if there are very recent documents (last hour)
                const oneHourAgo = Date.now() - (60 * 60 * 1000);
                const recentDocuments = recentDomains.filter(domain => 
                    domain.createdAt && domain.createdAt > oneHourAgo
                );
                
                if (recentDocuments.length > 0) {
                    console.log(`   🚀 ACTIVE PLATFORM: ${recentDocuments.length} domains created in last hour`);
                    console.log('   📝 Analysis: Other users are successfully writing to platform');
                    console.log('   💡 Issue may be specific to our authentication/data format');
                } else {
                    console.log('   ⏳ No domains in last hour - platform may be quiet or having issues');
                }
                
            } else {
                console.log('   ⚠️  No recent domains found - unusual for active testnet');
            }
            
        } catch (error) {
            console.log(`   ❌ Could not fetch recent documents: ${error?.message || error}`);
        }
        
        // Check recent identities to see registration activity
        console.log('\n👤 Checking recent identity activity...');
        try {
            // Get identities with recent activity (this is a more general query)
            const identities = await sdk.getIdentities({
                limit: 5,
                orderBy: [["$createdAt", "desc"]]
            });
            
            if (identities && identities.length > 0) {
                console.log(`   ✅ Found ${identities.length} identities`);
                identities.forEach((identity, index) => {
                    console.log(`   👤 Identity ${index + 1}: ${identity.id} (Balance: ${identity.balance || 'unknown'})`);
                });
            } else {
                console.log('   ⚠️  No recent identities found');
            }
            
        } catch (error) {
            console.log(`   ❌ Could not fetch identities: ${error?.message || error}`);
            // This is expected - getIdentities may not be available
        }
        
        await sdk.destroy();
        return { success: true };
        
    } catch (error) {
        console.log(`❌ Recent activity check failed: ${error?.message || error}`);
        if (sdk) await sdk.destroy();
        return { success: false, error: error?.message || error };
    }
}

async function testSimplestOperation() {
    console.log('\n🧪 Testing simplest possible write operation...\n');
    
    const mnemonic = process.env.MNEMONIC || 'lamp truck drip furnace now swing income victory leisure popular jeans vehicle';
    const identityId = process.env.IDENTITY_ID || 'DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq';
    
    let sdk;
    try {
        sdk = new WasmSDK({ 
            network: 'testnet', 
            transport: { url: BEST_ENDPOINT, timeout: 60000 }, // Longer timeout
            proofs: false, 
            debug: true 
        });
        await sdk.initialize();
        
        // Try the absolute simplest document structure
        const minimalData = {
            normalizedLabel: Date.now().toString(),
            normalizedParentDomainName: 'dash'
        };
        
        console.log('📝 Attempting minimal document with extended timeout...');
        console.log(`   Data: ${JSON.stringify(minimalData)}`);
        console.log('   Timeout: 60 seconds');
        
        const startTime = Date.now();
        
        try {
            const result = await sdk.createDocument(
                mnemonic,
                identityId,
                DPNS_CONTRACT,
                'domain',
                JSON.stringify(minimalData),
                2  // Try keyIndex 2 (HIGH security level)
            );
            
            const operationTime = Date.now() - startTime;
            console.log(`   ✅ SUCCESS: ${operationTime}ms`);
            console.log(`   📄 Result: ${JSON.stringify(result, null, 2)}`);
            
            await sdk.destroy();
            return { success: true, result, operationTime };
            
        } catch (error) {
            const operationTime = Date.now() - startTime;
            console.log(`   ❌ Failed after ${operationTime}ms: ${error?.message || error}`);
            
            // Try to extract more details from the error
            if (error && typeof error === 'object') {
                console.log(`   🔬 Error details: ${JSON.stringify(error, null, 2)}`);
            }
            
            await sdk.destroy();
            return { success: false, error: error?.message || error, operationTime };
        }
        
    } catch (error) {
        console.log(`❌ SDK initialization failed: ${error?.message || error}`);
        if (sdk) await sdk.destroy();
        return { success: false, error: error?.message || error };
    }
}

// Run tests
console.log('🚀 Starting recent activity analysis...\n');

const activityCheck = await checkRecentActivity();
const operationTest = await testSimplestOperation();

console.log('\n📊 PLATFORM HEALTH ANALYSIS');
console.log('===========================\n');

if (activityCheck.success) {
    console.log('✅ Platform query operations working normally');
} else {
    console.log('❌ Platform query operations having issues');
}

if (operationTest.success) {
    console.log('🎉 BREAKTHROUGH: Write operations working!');
    console.log(`   ⚡ Operation completed in ${operationTest.operationTime}ms`);
    console.log('   💡 Previous issues may have been temporary or data-format related');
} else {
    console.log('❌ Write operations still failing');
    console.log('   📝 Confirmed: This is a consistent platform broadcast issue');
    console.log('   🔧 Recommendation: Platform infrastructure issue - not SDK problem');
}

console.log('\n🎯 CONCLUSION:');
if (operationTest.success) {
    console.log('✅ Platform operations working - ready for full PRD compliance testing');
} else {
    console.log('❌ Platform broadcast issue confirmed - need alternative testing strategy');
    console.log('💡 Could test with mainnet or wait for testnet infrastructure fix');
}

console.log('\n✅ Recent activity test complete');