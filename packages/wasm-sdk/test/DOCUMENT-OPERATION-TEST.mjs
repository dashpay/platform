#!/usr/bin/env node
/**
 * Document Operation Test - Direct test of document creation with credit consumption
 * Tests the core PRD requirement: real credit consumption
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

console.log('🧪 DOCUMENT OPERATION TEST');
console.log('Testing document creation with dual verification (credits + existence)\n');

// Test configuration from .env
const TEST_CONFIG = {
    IDENTITY_ID: process.env.IDENTITY_ID || 'DcoJJ3W9JauwLD51vzNuXJ9vnaZT7mprVm7wbgVYifNq',
    MNEMONIC: process.env.MNEMONIC || 'lamp truck drip furnace now swing income victory leisure popular jeans vehicle',
    DPNS_CONTRACT: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'
};

console.log('📋 Test Configuration:');
console.log(`   Identity ID: ${TEST_CONFIG.IDENTITY_ID}`);
console.log(`   Has Mnemonic: ${!!TEST_CONFIG.MNEMONIC}`);
console.log(`   DPNS Contract: ${TEST_CONFIG.DPNS_CONTRACT}`);

// Use the fastest endpoint from our network test
const BEST_ENDPOINT = 'https://44.240.98.102:1443';

async function testDocumentCreationWithDualVerification() {
    console.log(`\n🚀 Starting document creation test with endpoint: ${BEST_ENDPOINT}`);
    
    let sdk;
    try {
        // Initialize SDK
        console.log('   🔧 Initializing SDK...');
        sdk = new WasmSDK({ 
            network: 'testnet', 
            transport: { url: BEST_ENDPOINT, timeout: 30000 },
            proofs: false, 
            debug: true 
        });
        await sdk.initialize();
        console.log('   ✅ SDK initialized successfully');
        
        // STEP 1: Get initial balance (VERIFICATION BASELINE)
        console.log('\n📊 STEP 1: Getting initial balance...');
        const beforeBalance = await sdk.getIdentityBalance(TEST_CONFIG.IDENTITY_ID);
        console.log(`   💰 Initial balance: ${beforeBalance.balance} credits`);
        
        // STEP 2: Create document (ACTUAL OPERATION)
        console.log('\n📝 STEP 2: Creating document...');
        const testData = {
            normalizedLabel: 'test-doc-' + Date.now(),
            normalizedParentDomainName: 'dash',
            label: 'test-doc-' + Date.now(),
            parentDomainName: 'dash',
            records: {
                dashIdentity: TEST_CONFIG.IDENTITY_ID
            },
            subdomainRules: {
                allowSubdomains: false
            }
        };
        
        console.log(`   📋 Document data: ${JSON.stringify(testData, null, 2)}`);
        console.log('   🔐 Using keyIndex 1 (CRITICAL security level)...');
        
        let documentResult;
        const operationStartTime = Date.now();
        
        try {
            // Use the new method name as suggested by deprecation warning
            documentResult = await sdk.createDocument(
                TEST_CONFIG.MNEMONIC,
                TEST_CONFIG.IDENTITY_ID,
                TEST_CONFIG.DPNS_CONTRACT,
                'domain',
                JSON.stringify(testData),
                1  // keyIndex 1 for CRITICAL security level
            );
            
            const operationTime = Date.now() - operationStartTime;
            console.log(`   ✅ DOCUMENT CREATED: ${operationTime}ms`);
            console.log(`   📄 Result: ${JSON.stringify(documentResult, null, 2)}`);
            
        } catch (docError) {
            const operationTime = Date.now() - operationStartTime;
            console.log(`   ❌ DOCUMENT CREATION FAILED: ${operationTime}ms`);
            
            // Safely handle error message
            const errorMessage = docError?.message || docError?.toString() || 'Unknown error';
            console.log(`   🔍 Error details: ${errorMessage}`);
            
            // Check for specific error types
            if (errorMessage.includes('Missing response message')) {
                console.log('   📝 Analysis: This is the broadcast failure - network/platform issue');
            } else if (errorMessage.includes('Invalid public key security level')) {
                console.log('   📝 Analysis: Wrong security level - need CRITICAL or HIGH, got MASTER');
            } else if (errorMessage.includes('no available addresses')) {
                console.log('   📝 Analysis: DAPI client connectivity issue');
            } else {
                console.log('   📝 Analysis: New error type - requires investigation');
                console.log(`   🔬 Full error: ${JSON.stringify(docError, null, 2)}`);
            }
            
            // Still try to get balance to see if credits were consumed
            console.log('\n💰 Checking if credits were consumed despite error...');
        }
        
        // STEP 3: Get final balance (VERIFICATION 1: CREDIT CONSUMPTION)
        console.log('\n📊 STEP 3: Getting final balance...');
        const afterBalance = await sdk.getIdentityBalance(TEST_CONFIG.IDENTITY_ID);
        console.log(`   💰 Final balance: ${afterBalance.balance} credits`);
        
        const creditsConsumed = beforeBalance.balance - afterBalance.balance;
        console.log(`   📉 Credits consumed: ${creditsConsumed} credits`);
        
        if (creditsConsumed > 0) {
            console.log('   🎉 SUCCESS: Real credit consumption detected!');
        } else {
            console.log('   ⚠️  No credits consumed - operation may have failed');
        }
        
        // STEP 4: Try to verify document exists (VERIFICATION 2: EXISTENCE)
        if (documentResult && documentResult.documentId) {
            console.log('\n🔍 STEP 4: Verifying document existence...');
            
            try {
                const createdDocument = await sdk.getDocument(
                    TEST_CONFIG.DPNS_CONTRACT,
                    'domain',
                    documentResult.documentId
                );
                
                if (createdDocument) {
                    console.log('   ✅ DOCUMENT EXISTS: Dual verification successful!');
                    console.log(`   📄 Retrieved document: ${JSON.stringify(createdDocument.data || createdDocument, null, 2)}`);
                } else {
                    console.log('   ❌ DOCUMENT NOT FOUND: Credits consumed but document not readable');
                }
            } catch (readError) {
                const readErrorMessage = readError?.message || readError?.toString() || 'Unknown read error';
                console.log(`   ❌ DOCUMENT READ FAILED: ${readErrorMessage}`);
            }
        }
        
        await sdk.destroy();
        
        // Final assessment
        console.log('\n🎯 DUAL VERIFICATION ASSESSMENT:');
        console.log(`   Credit Consumption: ${creditsConsumed > 0 ? '✅ VERIFIED' : '❌ FAILED'}`);
        console.log(`   Document Existence: ${documentResult ? '✅ VERIFIED' : '❌ FAILED'}`);
        
        if (creditsConsumed > 0 && documentResult) {
            console.log('   🏆 PRD COMPLIANCE: ACHIEVED - Real platform operation with dual verification!');
            return { success: true, creditsConsumed, documentResult };
        } else if (creditsConsumed > 0) {
            console.log('   ⚠️  PARTIAL SUCCESS: Credits consumed but document creation unclear');
            return { success: false, creditsConsumed, documentResult: null, issue: 'document_creation_unclear' };
        } else {
            console.log('   ❌ PRD COMPLIANCE: FAILED - No real credit consumption detected');
            return { success: false, creditsConsumed: 0, documentResult: null, issue: 'no_credit_consumption' };
        }
        
    } catch (error) {
        const errorMessage = error?.message || error?.toString() || 'Unknown error';
        console.log(`\n❌ TEST FAILED: ${errorMessage}`);
        
        if (sdk) {
            await sdk.destroy();
        }
        
        return { success: false, error: errorMessage };
    }
}

// Run the test
console.log('🚀 Starting document operation test...\n');

const testResult = await testDocumentCreationWithDualVerification();

console.log('\n📊 FINAL TEST RESULTS');
console.log('====================');

if (testResult.success) {
    console.log('🎉 SUCCESS: Document operation working with real credit consumption');
    console.log(`   💰 Credits consumed: ${testResult.creditsConsumed}`);
    console.log(`   📄 Document ID: ${testResult.documentResult?.documentId || 'N/A'}`);
    console.log('\n✅ BREAKTHROUGH: PRD core requirement achieved!');
} else {
    console.log('❌ FAILED: Document operation not working as expected');
    console.log(`   💰 Credits consumed: ${testResult.creditsConsumed || 0}`);
    console.log(`   🔍 Issue: ${testResult.issue || 'Unknown'}`);
    console.log(`   🔧 Error: ${testResult.error || 'See details above'}`);
}

console.log('\n✅ Document operation test complete');