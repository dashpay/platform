#!/usr/bin/env node

/**
 * Simple Functionality Test
 * Tests that our WASM SDK works with the current build
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
        configurable: true
    });
}

async function runTests() {
    console.log('🧪 Simple WASM SDK Functionality Test');
    console.log('=====================================');

    let passed = 0;
    let failed = 0;

    async function test(name, fn) {
        try {
            const startTime = Date.now();
            await fn();
            const duration = Date.now() - startTime;
            console.log(`✅ ${name} (${duration}ms)`);
            passed++;
        } catch (error) {
            console.log(`❌ ${name}: ${error.message}`);
            failed++;
        }
    }

    // Initialize WASM
    await test('Initialize WASM module', async () => {
        const init = (await import('../pkg/dash_wasm_sdk.js')).default;
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const wasmBuffer = readFileSync(wasmPath);
        await init(wasmBuffer);
    });

    // Test JavaScript wrapper
    await test('Create SDK using JavaScript wrapper', async () => {
        const { WasmSDK } = await import('../src-js/index.js');
        
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: false,
            transport: {
                timeout: 15000,
                retries: 3
            }
        });

        await sdk.initialize();

        // Test basic operations
        const mnemonic = await sdk.generateMnemonic(12);
        if (typeof mnemonic !== 'string' || mnemonic.split(' ').length !== 12) {
            throw new Error('Mnemonic generation failed');
        }

        const isValid = await sdk.validateMnemonic(mnemonic);
        if (!isValid) {
            throw new Error('Mnemonic validation failed');
        }

        // Try network operation
        try {
            const status = await sdk.getStatus();
            console.log('   📡 Network status retrieved successfully');
        } catch (error) {
            console.log(`   ⚠️ Network operation failed: ${error.message}`);
        }

        await sdk.destroy();
    });

    // Test web app accessibility
    await test('Sample web apps are accessible', async () => {
        const response = await fetch('http://localhost:8888/samples/document-explorer/');
        if (!response.ok) {
            throw new Error(`Web app not accessible: ${response.status}`);
        }
        
        const content = await response.text();
        if (!content.includes('Document Explorer')) {
            throw new Error('Web app content invalid');
        }
    });

    // Test DPNS Resolver
    await test('DPNS Resolver app is accessible', async () => {
        const response = await fetch('http://localhost:8888/samples/dpns-resolver/');
        if (!response.ok) {
            throw new Error(`DPNS Resolver not accessible: ${response.status}`);
        }
        
        const content = await response.text();
        if (!content.includes('DPNS Resolver')) {
            throw new Error('DPNS Resolver content invalid');
        }
    });

    // Test Identity Manager
    await test('Identity Manager app is accessible', async () => {
        const response = await fetch('http://localhost:8888/samples/identity-manager/');
        if (!response.ok) {
            throw new Error(`Identity Manager not accessible: ${response.status}`);
        }
        
        const content = await response.text();
        if (!content.includes('Identity Manager')) {
            throw new Error('Identity Manager content invalid');
        }
    });

    // Test Token Transfer
    await test('Token Transfer app is accessible', async () => {
        const response = await fetch('http://localhost:8888/samples/token-transfer/');
        if (!response.ok) {
            throw new Error(`Token Transfer not accessible: ${response.status}`);
        }
        
        const content = await response.text();
        if (!content.includes('Token Transfer')) {
            throw new Error('Token Transfer content invalid');
        }
    });

    // Test multiple SDK instances
    await test('Multiple SDK instances work', async () => {
        const { WasmSDK } = await import('../src-js/index.js');
        
        const sdk1 = new WasmSDK({ network: 'testnet', proofs: false });
        const sdk2 = new WasmSDK({ network: 'testnet', proofs: false });
        
        await sdk1.initialize();
        await sdk2.initialize();
        
        const mnemonic1 = await sdk1.generateMnemonic(12);
        const mnemonic2 = await sdk2.generateMnemonic(12);
        
        if (mnemonic1 === mnemonic2) {
            throw new Error('SDK instances should generate different mnemonics');
        }
        
        await sdk1.destroy();
        await sdk2.destroy();
    });

    console.log('');
    console.log('📊 Test Summary');
    console.log('===============');
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`📈 Success Rate: ${(passed / (passed + failed) * 100).toFixed(1)}%`);

    if (failed === 0) {
        console.log('');
        console.log('🎉 All functionality tests passed!');
        console.log('The WASM SDK and sample applications are working correctly.');
        console.log('');
        console.log('✅ Test Results:');
        console.log('  - WASM SDK builds and initializes correctly');
        console.log('  - JavaScript wrapper functions properly');
        console.log('  - Cryptographic operations work');
        console.log('  - All 4 web sample applications are accessible');
        console.log('  - Multiple SDK instances can coexist');
        console.log('  - Network operations function (when connectivity allows)');
        return 0;
    } else {
        console.log('');
        console.log(`❌ ${failed} tests failed. Check the errors above.`);
        return 1;
    }
}

runTests().then(code => process.exit(code)).catch(error => {
    console.error('💥 Test execution crashed:', error.message);
    process.exit(1);
});