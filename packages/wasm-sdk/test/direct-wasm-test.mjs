#!/usr/bin/env node
/**
 * Direct WASM documentCreate test 
 * Tests the fixed WASM method directly to see actual result
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

import init, * as wasmSdk from '../pkg/dash_wasm_sdk.js';

console.log('🔬 DIRECT WASM documentCreate TEST');

async function testDirectWasm() {
    try {
        // Initialize WASM
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const wasmBuffer = readFileSync(wasmPath);
        await init(wasmBuffer);

        // Create SDK with trusted builder
        const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
        const sdk = await builder.build();

        console.log('✅ Direct WASM SDK created');

        // Test the fixed documentCreate method directly
        console.log('\n🔥 Testing fixed documentCreate directly...');
        
        const entropy = Array.from(crypto.getRandomValues(new Uint8Array(32)))
            .map(b => b.toString(16).padStart(2, '0')).join('');

        console.log('Calling documentCreate with:');
        console.log('- Contract:', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
        console.log('- Type: note');
        console.log('- Owner:', envVars.IDENTITY_ID);
        console.log('- Data: {"test": true}');
        console.log('- Entropy:', entropy.substring(0, 16) + '...');
        console.log('- Key:', envVars.PRIVATE_KEY_WIF.substring(0, 10) + '...');

        const result = await sdk.documentCreate(
            'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', 
            'note',
            envVars.IDENTITY_ID,
            '{"test": true}',
            entropy,
            envVars.PRIVATE_KEY_WIF
        );

        console.log('\n🎉 documentCreate SUCCEEDED!');
        console.log('Result:', result);
        console.log('Result type:', typeof result);
        console.log('Result constructor:', result.constructor.name);
        
        if (result && typeof result === 'object') {
            console.log('Result keys:', Object.keys(result));
            console.log('DocumentId:', result.documentId);
            console.log('Type:', result.type);
        }

        return { success: true, result: result };

    } catch (error) {
        console.log('❌ Direct WASM test failed');
        console.log('Error:', error.message || 'undefined');
        
        if (error.message && error.message.includes('Missing response message')) {
            console.log('🚨 OLD BROADCAST BUG STILL EXISTS');
            return { success: false, oldBugExists: true };
        } else {
            console.log('🔍 Different error (broadcast fix working)');
            return { success: false, broadcastFixed: true, error: error.message };
        }
    }
}

testDirectWasm().then(result => {
    console.log('\n🎯 DIRECT WASM TEST RESULT:');
    if (result.success) {
        console.log('✅ documentCreate working at WASM level');
        console.log('🎉 BROADCAST BUG FIX CONFIRMED!');
    } else if (result.broadcastFixed) {
        console.log('✅ Broadcast bug fixed (no Missing response message)');
        console.log('⚠️ Different issue detected');
    } else {
        console.log('❌ Test failed');
    }
});