#!/usr/bin/env node
/**
 * Network Diagnostic Test - Debug platform network connectivity issues
 * Tests different DAPI endpoints to resolve "Missing response message" errors
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

console.log('🔍 NETWORK DIAGNOSTIC TEST');
console.log('Testing different DAPI endpoints to resolve connectivity issues\n');

// Test configuration
const IDENTITY_ID = process.env.IDENTITY_ID || 'DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq';

// Primary testnet endpoints from the code
const TESTNET_ENDPOINTS = [
    'https://52.12.176.90:1443',      // Primary endpoint from code
    'https://35.82.197.197:1443',     // Secondary endpoint
    'https://44.240.98.102:1443',     // Third endpoint
    'https://52.34.144.50:1443',      // Fourth endpoint
    'https://44.239.39.153:1443',     // Fifth endpoint
];

// Alternative endpoints to try if main ones fail
const ALTERNATIVE_ENDPOINTS = [
    'https://35.164.23.245:1443',
    'https://54.149.33.167:1443',
];

async function testEndpointConnectivity(endpoint) {
    console.log(`🔗 Testing endpoint: ${endpoint}`);
    
    try {
        // Create SDK with custom endpoint
        const sdk = new WasmSDK({ 
            network: 'testnet', 
            transport: { url: endpoint, timeout: 10000 },
            proofs: false, 
            debug: true 
        });
        
        await sdk.initialize();
        
        // Test basic connectivity - get identity balance
        const startTime = Date.now();
        const identity = await sdk.getIdentityBalance(IDENTITY_ID);
        const responseTime = Date.now() - startTime;
        
        console.log(`   ✅ SUCCESS: ${responseTime}ms - Balance: ${identity.balance} credits`);
        
        await sdk.destroy();
        return { 
            endpoint, 
            success: true, 
            responseTime, 
            balance: identity.balance,
            error: null 
        };
        
    } catch (error) {
        console.log(`   ❌ FAILED: ${error.message}`);
        return { 
            endpoint, 
            success: false, 
            responseTime: null, 
            balance: null,
            error: error.message 
        };
    }
}

async function testDocumentOperation(workingEndpoint) {
    console.log(`\n🧪 Testing document operation with working endpoint: ${workingEndpoint}`);
    
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const MNEMONIC = process.env.MNEMONIC || 'lamp truck drip furnace now swing income victory leisure popular jeans vehicle';
    
    try {
        const sdk = new WasmSDK({ 
            network: 'testnet', 
            transport: { url: workingEndpoint, timeout: 30000 },
            proofs: false, 
            debug: true 
        });
        
        await sdk.initialize();
        
        console.log('   📊 Testing document creation...');
        
        // Test document creation with keyIndex 1 (CRITICAL security level)
        const testData = {
            message: 'Network diagnostic test',
            timestamp: Date.now(),
            endpoint: workingEndpoint
        };
        
        const startTime = Date.now();
        
        try {
            const result = await sdk.documentCreate(
                MNEMONIC,
                IDENTITY_ID, 
                DPNS_CONTRACT,
                'domain',
                JSON.stringify(testData),
                1  // Use keyIndex 1 for CRITICAL security level
            );
            
            const operationTime = Date.now() - startTime;
            console.log(`   ✅ DOCUMENT OPERATION SUCCESS: ${operationTime}ms`);
            console.log(`      Result: ${JSON.stringify(result, null, 2)}`);
            
            await sdk.destroy();
            return { success: true, operationTime, result };
            
        } catch (opError) {
            const operationTime = Date.now() - startTime;
            console.log(`   ⚠️  DOCUMENT OPERATION DETAILS: ${operationTime}ms`);
            console.log(`      Error: ${opError.message}`);
            
            // Check if this is the specific broadcast error we're debugging
            if (opError.message.includes('Missing response message')) {
                console.log('   🔍 CONFIRMED: This is the "Missing response message" error');
                console.log('   📝 Analysis: Authentication working, state transition created, broadcast failing');
            } else if (opError.message.includes('no available addresses')) {
                console.log('   🔍 CONFIRMED: This is the "no available addresses" error');
                console.log('   📝 Analysis: DAPI client connectivity issue');
            }
            
            await sdk.destroy();
            return { success: false, operationTime, error: opError.message };
        }
        
    } catch (error) {
        console.log(`   ❌ SDK INITIALIZATION FAILED: ${error.message}`);
        return { success: false, error: error.message };
    }
}

async function runNetworkDiagnostics() {
    const results = {
        endpointTests: [],
        workingEndpoints: [],
        documentOperationTest: null,
        summary: {}
    };
    
    console.log('🔍 Phase 1: Testing primary endpoints...\n');
    
    // Test primary endpoints
    for (const endpoint of TESTNET_ENDPOINTS) {
        const result = await testEndpointConnectivity(endpoint);
        results.endpointTests.push(result);
        
        if (result.success) {
            results.workingEndpoints.push(endpoint);
        }
        
        // Small delay between tests
        await new Promise(resolve => setTimeout(resolve, 1000));
    }
    
    // If no primary endpoints work, try alternatives
    if (results.workingEndpoints.length === 0) {
        console.log('\n⚠️  Primary endpoints failed, trying alternatives...\n');
        
        for (const endpoint of ALTERNATIVE_ENDPOINTS) {
            const result = await testEndpointConnectivity(endpoint);
            results.endpointTests.push(result);
            
            if (result.success) {
                results.workingEndpoints.push(endpoint);
                break; // Found a working one, that's enough for now
            }
        }
    }
    
    // Test document operation if we have a working endpoint
    if (results.workingEndpoints.length > 0) {
        const fastestEndpoint = results.endpointTests
            .filter(r => r.success)
            .sort((a, b) => a.responseTime - b.responseTime)[0].endpoint;
            
        results.documentOperationTest = await testDocumentOperation(fastestEndpoint);
    }
    
    // Generate summary
    results.summary = {
        totalEndpointsTested: results.endpointTests.length,
        workingEndpoints: results.workingEndpoints.length,
        failedEndpoints: results.endpointTests.filter(r => !r.success).length,
        fastestEndpoint: results.workingEndpoints.length > 0 ? 
            results.endpointTests.filter(r => r.success).sort((a, b) => a.responseTime - b.responseTime)[0] : null,
        documentOperationSuccessful: results.documentOperationTest?.success || false
    };
    
    return results;
}

// Run diagnostics
console.log('🚀 Starting network diagnostics...\n');

const diagnostics = await runNetworkDiagnostics();

console.log('\n📊 NETWORK DIAGNOSTIC RESULTS');
console.log('===============================\n');

console.log('🔗 Endpoint Test Results:');
diagnostics.endpointTests.forEach(result => {
    if (result.success) {
        console.log(`   ✅ ${result.endpoint} - ${result.responseTime}ms (${result.balance} credits)`);
    } else {
        console.log(`   ❌ ${result.endpoint} - ${result.error}`);
    }
});

console.log(`\n📈 Summary:`);
console.log(`   • Total endpoints tested: ${diagnostics.summary.totalEndpointsTested}`);
console.log(`   • Working endpoints: ${diagnostics.summary.workingEndpoints}`);
console.log(`   • Failed endpoints: ${diagnostics.summary.failedEndpoints}`);

if (diagnostics.summary.fastestEndpoint) {
    console.log(`   • Fastest endpoint: ${diagnostics.summary.fastestEndpoint.endpoint} (${diagnostics.summary.fastestEndpoint.responseTime}ms)`);
} else {
    console.log(`   • No working endpoints found`);
}

if (diagnostics.documentOperationTest) {
    console.log(`\n🧪 Document Operation Test:`);
    if (diagnostics.documentOperationTest.success) {
        console.log(`   ✅ SUCCESS: Document operation completed in ${diagnostics.documentOperationTest.operationTime}ms`);
        console.log(`   🎉 BREAKTHROUGH: Platform operations working!`);
    } else {
        console.log(`   ❌ FAILED: ${diagnostics.documentOperationTest.error}`);
        console.log(`   📝 This confirms the specific issue blocking credit consumption`);
    }
}

console.log('\n🎯 NEXT STEPS:');
if (diagnostics.summary.workingEndpoints > 0 && diagnostics.documentOperationTest?.success) {
    console.log('   ✅ Network connectivity resolved - ready for full platform operations testing');
} else if (diagnostics.summary.workingEndpoints > 0) {
    console.log('   🔧 Basic connectivity working but document operations failing');
    console.log('   🔍 Need to debug specific broadcast/state transition issues');
} else {
    console.log('   🚨 No working endpoints found - network configuration issue');
    console.log('   🔧 Need to investigate DAPI endpoint accessibility');
}

console.log('\n✅ Network diagnostic complete');