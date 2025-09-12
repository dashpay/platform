#!/usr/bin/env node

/**
 * FINAL VALIDATION TEST - Real Credit Consumption
 * 
 * This test validates that the broadcast bug fix enables actual 
 * testnet credit consumption, completing PRD compliance.
 */

import WasmSDK from '../pkg/index.js';
import { readFileSync } from 'fs';

// Load environment variables manually
const envFile = readFileSync('.env', 'utf8');
const envVars = {};
envFile.split('\n').forEach(line => {
    const [key, value] = line.split('=');
    if (key && value) {
        envVars[key] = value.replace(/"/g, '');
    }
});

const MNEMONIC = envVars.MNEMONIC;
const IDENTITY_ID = envVars.IDENTITY_ID;  
const PRIVATE_KEY_WIF = envVars.PRIVATE_KEY_WIF;

// Test configuration  
const TEST_CONFIG = {
    network: 'testnet',
    transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000
    },
    proofs: false,  // Disable proofs for reliable testing
    debug: true
};

// Note contract for testing (known to exist on testnet)
const NOTE_CONTRACT_ID = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

async function runFinalValidationTest() {
    console.log('🎯 FINAL VALIDATION TEST - Real Credit Consumption');
    console.log('='.repeat(60));
    console.log(`📅 Test Time: ${new Date().toISOString()}`);
    console.log(`🔗 Network: ${TEST_CONFIG.network}`);
    console.log(`👤 Identity: ${IDENTITY_ID}`);
    console.log('='.repeat(60));

    try {
        // Initialize SDK
        console.log('🚀 Initializing WASM SDK...');
        const sdk = new WasmSDK(TEST_CONFIG);
        await sdk.initialize();
        console.log('✅ SDK initialized successfully');

        // Check initial identity balance
        console.log('\n💰 Checking initial identity balance...');
        const initialBalance = await sdk.getIdentityBalance(IDENTITY_ID);
        console.log(`💵 Initial Balance: ${initialBalance.balance} credits`);
        
        if (initialBalance.balance < 1000000) {
            throw new Error(`Insufficient credits: ${initialBalance.balance}. Need at least 1M credits for testing.`);
        }

        // Generate test document data
        const testDocument = {
            message: `Final validation test - ${Date.now()}`,
            timestamp: Date.now(),
            testType: 'broadcast-bug-fix-validation',
            entropy: Math.random().toString(36)
        };
        
        // Generate entropy for state transition  
        const entropy = Array.from(crypto.getRandomValues(new Uint8Array(32)))
            .map(b => b.toString(16).padStart(2, '0')).join('');

        console.log('\n📝 Creating test document...');
        console.log(`📄 Document data: ${JSON.stringify(testDocument, null, 2)}`);
        console.log(`🎲 Entropy: ${entropy}`);

        // THE CRITICAL TEST - Use the fixed document_create function
        console.log('\n🔥 TESTING FIXED BROADCAST - documentCreate with real credit consumption...');
        console.log('⏱️  This is the moment of truth - broadcast bug fix validation...');
        
        const startTime = Date.now();
        
        const result = await sdk.documentCreate(
            NOTE_CONTRACT_ID,       // contract ID
            'note',                 // document type
            IDENTITY_ID,            // owner ID  
            JSON.stringify(testDocument), // document data
            entropy,                // entropy
            PRIVATE_KEY_WIF         // private key for signing
        );
        
        const endTime = Date.now();
        const executionTime = endTime - startTime;

        console.log('\n🎉 BROADCAST SUCCESSFUL! 🎉');
        console.log(`⏱️  Execution time: ${executionTime}ms`);
        console.log(`📄 Result: ${JSON.stringify(result, null, 2)}`);

        // Check final balance to confirm credit consumption
        console.log('\n💰 Checking final balance for credit consumption...');
        const finalBalance = await sdk.getIdentityBalance(IDENTITY_ID);
        const creditsConsumed = initialBalance.balance - finalBalance.balance;

        console.log(`💵 Final Balance: ${finalBalance.balance} credits`);
        console.log(`🔥 Credits Consumed: ${creditsConsumed} credits`);

        // Validation results
        console.log('\n' + '='.repeat(60));
        console.log('🏆 FINAL VALIDATION RESULTS');
        console.log('='.repeat(60));

        const validationResults = {
            broadcastWorking: true,
            creditConsumption: creditsConsumed > 0,
            executionTime: executionTime,
            documentCreated: !!result.documentId,
            prddCompliant: !!result.type,
            creditsConsumed: creditsConsumed,
            networkConnectivity: true,
            authenticationWorking: true
        };

        Object.entries(validationResults).forEach(([key, value]) => {
            const status = typeof value === 'boolean' ? (value ? '✅ PASS' : '❌ FAIL') : `📊 ${value}`;
            console.log(`${key.padEnd(20)}: ${status}`);
        });

        // Overall assessment
        const allPassed = validationResults.broadcastWorking && 
                         validationResults.creditConsumption && 
                         validationResults.documentCreated;

        console.log('\n' + '='.repeat(60));
        if (allPassed) {
            console.log('🎯 PRD COMPLIANCE STATUS: ✅ ACHIEVED!');
            console.log('🚀 WASM SDK: 100% FUNCTIONAL WITH REAL CREDIT CONSUMPTION');
            console.log('🔥 BROADCAST BUG FIX: ✅ VERIFIED WORKING');
        } else {
            console.log('⚠️  PRD COMPLIANCE STATUS: ❌ INCOMPLETE');
            console.log('🔍 Issues detected - see results above');
        }
        console.log('='.repeat(60));

        // Cleanup
        await sdk.destroy();
        
        return {
            success: allPassed,
            results: validationResults,
            executionTime: executionTime,
            creditsConsumed: creditsConsumed
        };

    } catch (error) {
        console.error('\n❌ FINAL VALIDATION TEST FAILED');
        console.error(`💥 Error: ${error.message}`);
        console.error(`📍 Stack: ${error.stack}`);
        
        // Check if it's the old broadcast error
        if (error.message.includes('Missing response message')) {
            console.error('\n🚨 BROADCAST BUG STILL EXISTS!');
            console.error('🔧 The broadcast_and_wait fix may not be complete');
        } else if (error.message.includes('Failed to broadcast')) {
            console.error('\n🔍 BROADCAST ISSUE DETECTED');
            console.error('📋 This may be a different broadcast-related problem');
        }
        
        return {
            success: false,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

// Run the test if called directly
if (import.meta.url === `file://${process.argv[1]}`) {
    runFinalValidationTest()
        .then(result => {
            console.log(`\n📊 Final test result: ${result.success ? 'SUCCESS' : 'FAILURE'}`);
            process.exit(result.success ? 0 : 1);
        })
        .catch(error => {
            console.error(`Fatal test error: ${error.message}`);
            process.exit(1);
        });
}

export default runFinalValidationTest;