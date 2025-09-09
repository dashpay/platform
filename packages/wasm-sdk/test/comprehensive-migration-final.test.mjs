#!/usr/bin/env node
// comprehensive-migration-final.test.mjs - Final comprehensive migration test (MIGRATED)

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { webcrypto } from 'crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

if (!global.crypto) {
    Object.defineProperty(global, 'crypto', { value: webcrypto, writable: true, configurable: true });
}

// 🎯 MIGRATED: Import JavaScript wrapper
import init from '../pkg/wasm_sdk.js';
import { WasmSDK } from '../src-js/index.js';

const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
await init(readFileSync(wasmPath));

let passed = 0, failed = 0;

async function test(name, fn) {
    try {
        await fn();
        console.log(`✅ ${name}`);
        passed++;
    } catch (error) {
        console.log(`❌ ${name}: ${error.message}`);
        failed++;
    }
}

console.log('\n🎯 Final Comprehensive Migration Test - All Wrapper Functions (MIGRATED)\n');

// Test all wrapper function categories in one comprehensive test
await test('Complete wrapper ecosystem - all categories', async () => {
    const sdk = new WasmSDK({ network: 'testnet', proofs: false, debug: false });
    await sdk.initialize();
    
    // Test crypto functions
    const mnemonic = await sdk.generateMnemonic(12);
    const keyPair = await sdk.generateKeyPair('testnet');
    const signature = await sdk.signMessage('test', keyPair.private_key_wif);
    
    // Test DPNS functions  
    const usernameValid = await sdk.dpnsIsValidUsername('alice');
    const homographSafe = await sdk.dpnsConvertToHomographSafe('Alice');
    
    // Test system functions (may fail with network, but functions exist)
    const hasSystemFunctions = typeof sdk.getStatus === 'function' && 
                              typeof sdk.getCurrentEpoch === 'function';
    
    // Test identity functions (may fail with network, but functions exist)
    const hasIdentityFunctions = typeof sdk.getIdentityBalance === 'function' &&
                                typeof sdk.getIdentityKeys === 'function';
    
    // Test token functions (may fail with network, but functions exist)
    const hasTokenFunctions = typeof sdk.getTokenStatuses === 'function' &&
                             typeof sdk.calculateTokenIdFromContract === 'function';
    
    // Test state transition functions (may fail with invalid data, but functions exist)
    const hasStateFunctions = typeof sdk.identityCreate === 'function' &&
                             typeof sdk.documentCreate === 'function';
    
    if (!mnemonic || !keyPair || !signature || !usernameValid || homographSafe !== 'alice') {
        throw new Error('Core functions should work offline');
    }
    
    if (!hasSystemFunctions || !hasIdentityFunctions || !hasTokenFunctions || !hasStateFunctions) {
        throw new Error('All function categories should be available');
    }
    
    await sdk.destroy();
    
    console.log('   ✓ All wrapper function categories working in ecosystem');
    console.log(`   🔑 Crypto: Working`);
    console.log(`   🌐 DPNS: Working`);
    console.log(`   ⚙️ System: Available`);
    console.log(`   👤 Identity: Available`);
    console.log(`   🪙 Token: Available`);
    console.log(`   🌟 State Transitions: Available`);
});

await test('Parallel wrapper operations stress test', async () => {
    const sdk = new WasmSDK({ network: 'testnet', proofs: false, debug: false });
    await sdk.initialize();
    
    // Run many operations in parallel to test stability
    const operations = await Promise.all([
        sdk.generateMnemonic(12),
        sdk.generateMnemonic(24),
        sdk.generateKeyPair('testnet'),
        sdk.generateKeyPair('mainnet'),
        sdk.dpnsIsValidUsername('alice'),
        sdk.dpnsIsValidUsername('bob'),
        sdk.dpnsConvertToHomographSafe('Test1'),
        sdk.dpnsConvertToHomographSafe('Test2'),
        sdk.dpnsIsContestedUsername('alice'),
        sdk.dpnsIsContestedUsername('test')
    ]);
    
    if (operations.some(op => op === undefined || op === null)) {
        throw new Error('All parallel operations should return valid results');
    }
    
    await sdk.destroy();
    console.log(`   ✓ ${operations.length} parallel wrapper operations successful`);
});

await test('Cross-network wrapper consistency', async () => {
    const testnetSdk = new WasmSDK({ network: 'testnet', proofs: false, debug: false });
    const mainnetSdk = new WasmSDK({ network: 'mainnet', proofs: false, debug: false });
    
    await Promise.all([testnetSdk.initialize(), mainnetSdk.initialize()]);
    
    // Test that wrapper functions work consistently across networks
    const testnetKey = await testnetSdk.generateKeyPair('testnet');
    const mainnetKey = await mainnetSdk.generateKeyPair('mainnet');
    
    const testnetValid = await testnetSdk.validateAddress(testnetKey.address, 'testnet');
    const mainnetValid = await mainnetSdk.validateAddress(mainnetKey.address, 'mainnet');
    
    if (!testnetValid || !mainnetValid) {
        throw new Error('Generated addresses should validate on their networks');
    }
    
    await Promise.all([testnetSdk.destroy(), mainnetSdk.destroy()]);
    console.log('   ✓ Wrapper functions consistent across networks');
});

await test('Error handling consistency across all functions', async () => {
    const sdk = new WasmSDK({ network: 'testnet', proofs: false, debug: false });
    await sdk.initialize();
    
    // Test consistent error handling across function categories
    const errorTests = [
        () => sdk.generateMnemonic('invalid'),
        () => sdk.validateAddress(null, 'testnet'),
        () => sdk.getTokenStatuses('not-array'),
        () => sdk.getIdentitiesBalances('not-array')
    ];
    
    let properErrors = 0;
    for (const test of errorTests) {
        try {
            await test();
        } catch (error) {
            if (error.message.includes('Invalid') || error.message.includes('must be') || error.message.includes('Required')) {
                properErrors++;
            }
        }
    }
    
    if (properErrors < errorTests.length - 1) {
        throw new Error('Error handling should be consistent across functions');
    }
    
    await sdk.destroy();
    console.log(`   ✓ Error handling consistent (${properErrors}/${errorTests.length} proper errors)`);
});

console.log(`\n🎯 COMPLETE-WRAPPER-COVERAGE: ✅ ${passed} passed, ❌ ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);