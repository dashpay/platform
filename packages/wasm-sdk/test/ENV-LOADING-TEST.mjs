#!/usr/bin/env node
/**
 * ENV Loading Test - Validates .env file loading per PRD requirements
 * Tests that we can properly load credentials from .env as mandated by PRD
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

console.log('🔍 ENV FILE LOADING TEST');
console.log('Validating .env file loading per PRD requirements\\n');

// Method 1: Parse .env file manually (reliable)
console.log('1. Manual .env parsing:');
try {
    const envPath = join(__dirname, '../.env');
    const envContent = readFileSync(envPath, 'utf8');
    const envVars = {};

    envContent.split('\\n').forEach((line, index) => {
        line = line.trim();
        if (line && !line.startsWith('#') && line.includes('=')) {
            const equalIndex = line.indexOf('=');
            const key = line.substring(0, equalIndex).trim();
            let value = line.substring(equalIndex + 1).trim();
            
            if (value.startsWith('"') && value.endsWith('"')) {
                value = value.slice(1, -1);
            }
            
            envVars[key] = value;
            console.log(`  Line ${index}: ${key} = ${value}`);
        }
    });

    console.log('✅ Parsed variables:');
    console.log('  IDENTITY_ID:', envVars.IDENTITY_ID || 'MISSING');
    console.log('  MNEMONIC:', envVars.MNEMONIC ? envVars.MNEMONIC.split(' ').slice(0, 3).join(' ') + '...' : 'MISSING');
    console.log('  NETWORK:', envVars.NETWORK || 'MISSING');
    console.log('  PRIVATE_KEY_WIF:', envVars.PRIVATE_KEY_WIF ? 'Available' : 'MISSING');
    
    // Debug why some variables were missing
    console.log('\\n🔍 Debug missing variables:');
    Object.keys(envVars).forEach(key => {
        console.log(`  ${key}: ${envVars[key]}`);
    });

    // Set in process.env for testing
    Object.assign(process.env, envVars);
    
} catch (error) {
    console.log('❌ Manual parsing failed:', error.message);
}

// Method 2: Check process.env after setting
console.log('\\n2. Process.env validation:');
console.log('  process.env.IDENTITY_ID:', process.env.IDENTITY_ID || 'MISSING');
console.log('  process.env.MNEMONIC:', process.env.MNEMONIC ? 'Available' : 'MISSING');
console.log('  process.env.NETWORK:', process.env.NETWORK || 'MISSING');

// Method 3: Test with actual WASM SDK
console.log('\\n3. Testing with WASM SDK:');
try {
    const init = (await import('../pkg/dash_wasm_sdk.js')).default;
    const { WasmSDK } = await import('../src-js/index.js');
    
    const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
    await init(readFileSync(wasmPath));
    
    const sdk = new WasmSDK({ network: 'testnet', proofs: false });
    await sdk.initialize();
    
    // Test identity balance with loaded env vars
    const balance = await sdk.getIdentityBalance(process.env.IDENTITY_ID);
    console.log('✅ SDK can access identity with env vars');
    console.log('  Balance:', typeof balance === 'string' ? balance : balance.balance, 'credits');
    
    await sdk.destroy();
    
} catch (error) {
    console.log('❌ SDK test failed:', error.message);
}

console.log('\\n📋 ENV Loading Status:');
if (process.env.IDENTITY_ID && process.env.MNEMONIC) {
    console.log('✅ ENV file loading working correctly');
    console.log('✅ Ready for real credit consumption testing');
} else {
    console.log('❌ ENV file loading not working properly');
}

console.log('\\n✅ ENV loading test complete');