#!/usr/bin/env node

/**
 * Setup Verification Test
 * Quick test to verify WASM SDK and testing infrastructure works
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

console.log('🧪 WASM SDK Setup Verification');
console.log('==============================');

let passed = 0;
let failed = 0;

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

async function main() {
    // Test 1: WASM files exist
    await test('WASM files exist', async () => {
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const jsPath = join(__dirname, '../pkg/dash_wasm_sdk.js');
        
        const wasmExists = readFileSync(wasmPath).length > 0;
        const jsExists = readFileSync(jsPath, 'utf8').length > 0;
        
        if (!wasmExists) throw new Error('WASM binary missing or empty');
        if (!jsExists) throw new Error('WASM JavaScript bindings missing');
    });

    // Test 2: WASM module loads
    await test('WASM module loads', async () => {
        const init = (await import('../pkg/dash_wasm_sdk.js')).default;
        const wasmPath = join(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const wasmBuffer = readFileSync(wasmPath);
        await init(wasmBuffer);
    });

    // Test 3: JavaScript wrapper imports
    await test('JavaScript wrapper imports', async () => {
        const { WasmSDK } = await import('../src-js/index.js');
        if (typeof WasmSDK !== 'function') {
            throw new Error('WasmSDK is not a constructor function');
        }
    });

    // Test 4: SDK instance creation
    await test('SDK instance creation', async () => {
        const { WasmSDK } = await import('../src-js/index.js');
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: false
        });
        
        if (!sdk) throw new Error('Failed to create SDK instance');
        
        // Clean up
        if (sdk.destroy) await sdk.destroy();
    });

    // Test 5: SDK initialization
    await test('SDK initialization', async () => {
        const { WasmSDK } = await import('../src-js/index.js');
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: false,
            transport: { timeout: 10000 }
        });
        
        await sdk.initialize();
        
        if (sdk.destroy) await sdk.destroy();
    });

    // Test 6: Basic cryptographic operations
    await test('Basic cryptographic operations', async () => {
        const { WasmSDK } = await import('../src-js/index.js');
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: false
        });
        
        await sdk.initialize();
        
        // Generate mnemonic
        const mnemonic = await sdk.generateMnemonic(12);
        if (typeof mnemonic !== 'string') {
            throw new Error('Mnemonic generation failed');
        }
        
        if (mnemonic.split(' ').length !== 12) {
            throw new Error('Mnemonic should have 12 words');
        }
        
        // Validate mnemonic
        const isValid = await sdk.validateMnemonic(mnemonic);
        if (!isValid) {
            throw new Error('Generated mnemonic should be valid');
        }
        
        if (sdk.destroy) await sdk.destroy();
    });

    // Test 7: Network connectivity (optional)
    await test('Network connectivity (optional)', async () => {
        const { WasmSDK } = await import('../src-js/index.js');
        const sdk = new WasmSDK({
            network: 'testnet',
            proofs: false,
            debug: false,
            transport: { timeout: 15000 }
        });
        
        await sdk.initialize();
        
        try {
            const status = await sdk.getStatus();
            if (!status) {
                throw new Error('Network status should return data');
            }
        } catch (error) {
            // Network might be unavailable - that's OK for setup verification
            if (error.message.includes('fetch failed') || error.message.includes('timeout')) {
                console.log('   ⚠️ Network unavailable, but SDK is working');
            } else {
                throw error;
            }
        }
        
        if (sdk.destroy) await sdk.destroy();
    });

    // Test 8: Sample web apps exist
    await test('Sample web apps exist', async () => {
        const samplesPath = join(__dirname, '../samples');
        const samples = ['document-explorer', 'dpns-resolver', 'identity-manager', 'token-transfer'];
        
        for (const sample of samples) {
            const samplePath = join(samplesPath, sample, 'app.js');
            const appContent = readFileSync(samplePath, 'utf8');
            if (appContent.length === 0) {
                throw new Error(`Sample ${sample} app.js is empty`);
            }
        }
    });

    // Test 9: Examples exist and are accessible
    await test('Node.js examples exist', async () => {
        const examplesPath = join(__dirname, '../examples');
        const keyExamples = ['getting-started.mjs', 'identity-operations.mjs', 'contract-lookup.mjs'];
        
        for (const example of keyExamples) {
            const examplePath = join(examplesPath, example);
            const exampleContent = readFileSync(examplePath, 'utf8');
            if (exampleContent.length === 0) {
                throw new Error(`Example ${example} is empty`);
            }
        }
    });

    // Test 10: Test infrastructure files exist
    await test('Test infrastructure complete', async () => {
        const testFiles = [
            'unit/examples/getting-started.test.mjs',
            'unit/examples/identity-operations.test.mjs',
            'unit/examples/contract-lookup.test.mjs',
            'web-apps/document-explorer/functional.test.js',
            'web-apps/dpns-resolver/functionality.test.js',
            'integration/frameworks/framework-integration.test.mjs',
            'performance/load-testing.test.mjs'
        ];
        
        for (const testFile of testFiles) {
            const testPath = join(__dirname, testFile);
            const testContent = readFileSync(testPath, 'utf8');
            if (testContent.length === 0) {
                throw new Error(`Test file ${testFile} is empty`);
            }
        }
    });

    // Final summary
    console.log('');
    console.log('📊 Setup Verification Summary');
    console.log('============================');
    console.log(`✅ Passed: ${passed}`);
    console.log(`❌ Failed: ${failed}`);
    console.log(`📈 Success Rate: ${(passed / (passed + failed) * 100).toFixed(1)}%`);

    if (failed === 0) {
        console.log('');
        console.log('🎉 All setup verification tests passed!');
        console.log('The testing infrastructure is ready to use.');
        console.log('');
        console.log('Next steps:');
        console.log('  1. Run comprehensive tests: ./run-all-tests.sh');
        console.log('  2. Run specific test suites: npm run test:unit');
        console.log('  3. Run UI automation tests: cd ui-automation && npm test');
        return 0;
    } else {
        console.log('');
        console.log('❌ Some setup verification tests failed.');
        console.log('Please fix the issues above before running the full test suite.');
        return 1;
    }
}

main().then(code => process.exit(code)).catch(error => {
    console.error('💥 Setup verification crashed:', error.message);
    process.exit(1);
});