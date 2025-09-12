#!/usr/bin/env node
/**
 * Simple test to check if documentCreate works with the broadcast fix
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Set up crypto for Node.js
if (!global.crypto) {
    Object.defineProperty(global, 'crypto', {
        value: webcrypto,
        writable: true,
        configurable: true
    });
}

// Load .env
const envFile = readFileSync('.env', 'utf8');
const envVars = {};
envFile.split('\n').forEach(line => {
    if (line.includes('=') && !line.startsWith('#')) {
        const [key, value] = line.split('=');
        if (key && value) {
            envVars[key.trim()] = value.replace(/"/g, '').trim();
        }
    }
});

import init from '../pkg/dash_wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

console.log('🧪 SIMPLE DOCUMENT CREATE TEST');
console.log(`Identity: ${envVars.IDENTITY_ID}`);
console.log(`Network: ${envVars.NETWORK}`);

async function simpleTest() {
    try {
        // Initialize WASM
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const wasmBuffer = readFileSync(wasmPath);
        await init(wasmBuffer);

        // Create SDK  
        const sdk = new WasmSDK({
            network: envVars.NETWORK,
            proofs: false,
            debug: true
        });
        await sdk.initialize();

        console.log('✅ SDK initialized');

        // Test balance
        const balance = await sdk.getIdentityBalance(envVars.IDENTITY_ID);
        console.log(`💰 Balance: ${balance.balance} credits`);

        // Simple document test
        console.log('\n🔥 Testing documentCreate...');
        
        const result = await sdk.createDocument(
            envVars.MNEMONIC,
            envVars.IDENTITY_ID, 
            'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
            'note',
            '{"message":"test"}',
            0
        );

        console.log('🎉 SUCCESS! Document created without broadcast error!');
        console.log('Result type:', typeof result);
        console.log('Result keys:', Object.keys(result || {}));
        
        return { success: true, result };

    } catch (error) {
        console.log('❌ Test failed');
        console.log('Error message:', error.message || 'undefined');
        
        if (error.message && error.message.includes('Missing response message')) {
            console.log('🚨 OLD BROADCAST BUG DETECTED');
            return { success: false, oldBugDetected: true };
        } else {
            console.log('🔍 Different error - may be authentication or other issue');
            return { success: false, differentError: true, error: error.message };
        }
    }
}

simpleTest().then(result => {
    console.log('\nResult:', result.success ? '✅ SUCCESS' : '❌ FAILED');
});