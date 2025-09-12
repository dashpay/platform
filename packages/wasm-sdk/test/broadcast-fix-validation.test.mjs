#!/usr/bin/env node
/**
 * BROADCAST FIX VALIDATION TEST
 * Tests the fixed document_create function with Node.js compatibility
 * Uses .env file credentials as required by PRD
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

// Get directory paths
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Set up globals for WASM (Node.js compatibility fix)
if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

// Load environment variables from .env file (PRD requirement)
const envPath = join(__dirname, '../.env');
const envFile = readFileSync(envPath, 'utf8');
const envVars = {};
envFile.split('\n').forEach(line => {
    if (line.includes('=') && !line.startsWith('#')) {
        const [key, value] = line.split('=');
        if (key && value) {
            envVars[key.trim()] = value.replace(/"/g, '').trim();
        }
    }
});

// Import WASM and JavaScript wrapper
import init, * as wasmSdk from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

console.log('🔥 BROADCAST FIX VALIDATION TEST');
console.log('='.repeat(60));
console.log('📅 Test Time:', new Date().toISOString());
console.log('🔗 Network:', envVars.NETWORK);
console.log('👤 Identity:', envVars.IDENTITY_ID);
console.log('='.repeat(60));

async function validateBroadcastFix() {
    try {
        // Step 1: Initialize WASM module (Node.js compatible way)
        console.log('\n🧪 Step 1: WASM Module Initialization...');
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const wasmBuffer = readFileSync(wasmPath);
        await init(wasmBuffer);
        console.log('✅ WASM module initialized successfully');

        // Step 2: Initialize JavaScript wrapper (working approach from state-transitions.test.mjs)
        console.log('\n🧪 Step 2: JavaScript Wrapper Initialization...');
        const sdk = new WasmSDK({
            network: envVars.NETWORK,
            proofs: false,  // Disable proofs to avoid testnet quorum issues
            debug: true
        });
        await sdk.initialize();
        console.log('✅ JavaScript wrapper initialized successfully');

        // Step 3: Test identity balance (validates connectivity)
        console.log('\n🧪 Step 3: Network Connectivity Test...');
        const balance = await sdk.getIdentityBalance(envVars.IDENTITY_ID);
        console.log(`✅ Identity balance: ${balance.balance} credits`);
        
        // Step 4: Test the fixed documentCreate method 
        console.log('\n🧪 Step 4: BROADCAST FIX TEST - documentCreate...');
        
        const testDocument = {
            message: `Broadcast fix test - ${Date.now()}`,
            timestamp: Date.now(),
            testType: 'node-js-broadcast-validation'
        };

        console.log('📄 Test document:', JSON.stringify(testDocument, null, 2));
        
        // This is the critical test - the fixed documentCreate method via JavaScript wrapper
        console.log('\n🔥 TESTING FIXED BROADCAST METHOD...');
        const startTime = Date.now();
        
        const result = await sdk.createDocument(
            envVars.MNEMONIC,                // mnemonic from .env (PRD requirement)
            envVars.IDENTITY_ID,             // identity ID from .env
            'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', // note contract
            'note',                          // document type
            JSON.stringify(testDocument),    // document data
            0                                // key index
        );
        
        const endTime = Date.now();
        console.log(`⚡ Execution time: ${endTime - startTime}ms`);
        console.log('📊 Result:', JSON.stringify(result, null, 2));

        // Step 5: Validate result structure  
        console.log('\n🧪 Step 5: Result Validation...');
        const validations = {
            hasDocumentId: !!result.documentId,
            hasType: !!result.type,
            typeCorrect: result.type === 'DocumentCreated',
            hasCreatedFlag: !!result.created,
            executionTime: endTime - startTime
        };

        console.log('\n📊 VALIDATION RESULTS:');
        Object.entries(validations).forEach(([key, value]) => {
            const status = typeof value === 'boolean' ? (value ? '✅' : '❌') : `📊 ${value}ms`;
            console.log(`${key.padEnd(20)}: ${status}`);
        });

        // Overall assessment
        const broadcastWorking = validations.hasDocumentId && validations.typeCorrect;
        
        console.log('\n' + '='.repeat(60));
        if (broadcastWorking) {
            console.log('🎯 BROADCAST BUG FIX: ✅ VERIFIED WORKING!');
            console.log('🚀 documentCreate successfully completed without "Missing response message" error');
            console.log('📈 PRD COMPLIANCE: Ready for credit consumption validation');
        } else {
            console.log('❌ BROADCAST BUG FIX: VALIDATION FAILED');
            console.log('🔧 Issue persists or different problem detected');
        }
        console.log('='.repeat(60));

        return {
            success: broadcastWorking,
            results: validations,
            documentId: result.documentId
        };

    } catch (error) {
        console.error('\n❌ BROADCAST FIX VALIDATION FAILED');
        console.error('💥 Error:', error.message);
        
        if (error.message && error.message.includes('Missing response message')) {
            console.error('🚨 BROADCAST BUG STILL EXISTS!');
        } else if (error.message && error.message.includes('fetch')) {
            console.error('🔧 Node.js fetch compatibility issue detected');
        } else if (!error.message) {
            console.error('🔍 Undefined error - may indicate successful execution with result handling issue');
        }
        
        console.error('📍 Stack:', error.stack);
        
        return {
            success: false,
            error: error.message
        };
    }
}

// Run the validation
validateBroadcastFix()
    .then(result => {
        console.log(`\n📊 FINAL RESULT: ${result.success ? '✅ SUCCESS' : '❌ FAILURE'}`);
        process.exit(result.success ? 0 : 1);
    })
    .catch(error => {
        console.error(`Fatal error: ${error.message}`);
        process.exit(1);
    });