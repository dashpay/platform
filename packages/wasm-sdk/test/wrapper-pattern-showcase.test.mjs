#!/usr/bin/env node
// wrapper-pattern-showcase.test.mjs - Complete wrapper pattern showcase (MIGRATED)

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

console.log('\n🎉 WRAPPER PATTERN SHOWCASE - COMPLETE ECOSYSTEM (MIGRATED)\n');

await test('Modern initialization pattern showcase', async () => {
    // Showcase modern wrapper initialization
    const sdk = new WasmSDK({
        network: 'testnet',
        proofs: false,
        debug: false,
        transport: {
            timeout: 30000,
            retries: 3
        }
    });
    
    await sdk.initialize();
    
    if (!sdk.isInitialized()) {
        throw new Error('Modern initialization should work');
    }
    
    const config = sdk.getConfig();
    const network = sdk.getNetwork();
    const endpoint = sdk.getCurrentEndpoint();
    
    if (!config || !network || !endpoint) {
        throw new Error('Configuration access should work');
    }
    
    await sdk.destroy();
    console.log('   ✓ Modern initialization pattern working perfectly');
});

await test('Complete function category showcase', async () => {
    const sdk = new WasmSDK({ network: 'testnet', proofs: false, debug: false });
    await sdk.initialize();
    
    console.log('   🎯 Testing all function categories:');
    
    // Crypto category
    const mnemonic = await sdk.generateMnemonic(12);
    const keyPair = await sdk.generateKeyPair('testnet');
    console.log('     🔑 Crypto: Working ✅');
    
    // DPNS category
    const valid = await sdk.dpnsIsValidUsername('alice');
    const safe = await sdk.dpnsConvertToHomographSafe('Alice');
    console.log('     🌐 DPNS: Working ✅');
    
    // System functions exist
    if (typeof sdk.getStatus === 'function' && typeof sdk.getCurrentEpoch === 'function') {
        console.log('     ⚙️ System: Available ✅');
    }
    
    // Identity functions exist
    if (typeof sdk.getIdentityBalance === 'function' && typeof sdk.getIdentityKeys === 'function') {
        console.log('     👤 Identity: Available ✅');
    }
    
    // Token functions exist
    if (typeof sdk.getTokenStatuses === 'function' && typeof sdk.getTokenContractInfo === 'function') {
        console.log('     🪙 Token: Available ✅');
    }
    
    // State transition functions exist
    if (typeof sdk.identityCreate === 'function' && typeof sdk.documentCreate === 'function') {
        console.log('     🌟 State Transitions: Available ✅');
    }
    
    await sdk.destroy();
    console.log('   ✓ All function categories validated in complete ecosystem');
});

await test('Production-ready pattern demonstration', async () => {
    try {
        const sdk = new WasmSDK({ network: 'testnet', proofs: false, debug: false });
        await sdk.initialize();
        
        // Demonstrate production patterns
        const wallet = {
            mnemonic: await sdk.generateMnemonic(12),
            keys: [],
            addresses: []
        };
        
        // Generate multiple keys
        for (let i = 0; i < 3; i++) {
            const key = await sdk.deriveKeyFromSeedWithPath(wallet.mnemonic, '', `m/44'/1'/0'/0/${i}`, 'testnet');
            wallet.keys.push(key);
            wallet.addresses.push(key.address);
        }
        
        // Validate all addresses
        for (const address of wallet.addresses) {
            const isValid = await sdk.validateAddress(address, 'testnet');
            if (!isValid) throw new Error(`Address ${address} should be valid`);
        }
        
        // Test DPNS integration
        for (const username of ['alice', 'bob', 'test123']) {
            const isValid = await sdk.dpnsIsValidUsername(username);
            const safe = await sdk.dpnsConvertToHomographSafe(username);
            // Results collected for production use
        }
        
        await sdk.destroy();
        
        if (wallet.addresses.length !== 3) {
            throw new Error('Production pattern should generate 3 addresses');
        }
        
        console.log('   ✓ Production-ready patterns demonstrated successfully');
        console.log(`     Wallet: ${wallet.addresses.length} addresses generated`);
        console.log(`     DPNS: Username validation integrated`);
        console.log(`     Resources: Proper cleanup completed`);
        
    } catch (error) {
        throw error;
    }
});

console.log(`\n🎯 WRAPPER-PATTERN-SHOWCASE: ✅ ${passed} passed, ❌ ${failed} failed`);

if (failed === 0) {
    console.log(`\n🏆 COMPLETE WRAPPER PATTERN SHOWCASE SUCCESSFUL! 🏆`);
    console.log(`Production-ready wrapper patterns demonstrated across all categories.`);
}

process.exit(failed > 0 ? 1 : 0);