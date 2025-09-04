/**
 * Basic test for the JavaScript wrapper
 * Tests configuration management, error handling, and basic initialization
 */

import { WasmSDK, ConfigUtils, WasmConfigurationError } from '../index.js';

console.log('🧪 Running WASM SDK Wrapper Tests...\n');

let testCount = 0;
let passedCount = 0;
let failedCount = 0;

function test(description, testFn) {
    testCount++;
    console.log(`Test ${testCount}: ${description}`);
    
    try {
        testFn();
        passedCount++;
        console.log('  ✅ PASSED\n');
    } catch (error) {
        failedCount++;
        console.log(`  ❌ FAILED: ${error.message}\n`);
    }
}

async function asyncTest(description, testFn) {
    testCount++;
    console.log(`Test ${testCount}: ${description}`);
    
    try {
        await testFn();
        passedCount++;
        console.log('  ✅ PASSED\n');
    } catch (error) {
        failedCount++;
        console.log(`  ❌ FAILED: ${error.message}\n`);
    }
}

// Test 1: Basic instantiation
test('WasmSDK can be instantiated with default config', () => {
    const sdk = new WasmSDK();
    
    if (!sdk) {
        throw new Error('SDK instance not created');
    }
    
    if (sdk.isInitialized()) {
        throw new Error('SDK should not be initialized immediately');
    }
    
    const config = sdk.getConfig();
    if (config.network !== 'testnet') {
        throw new Error(`Expected network 'testnet', got '${config.network}'`);
    }
});

// Test 2: Configuration validation
test('Configuration validation works correctly', () => {
    // Valid configuration should not throw
    const validConfig = {
        network: 'testnet',
        transport: {
            timeout: 30000
        },
        proofs: true
    };
    
    const sdk1 = new WasmSDK(validConfig);
    const config1 = sdk1.getConfig();
    
    if (config1.network !== 'testnet') {
        throw new Error('Network not set correctly');
    }
    
    if (config1.transport.timeout !== 30000) {
        throw new Error('Timeout not set correctly');
    }
    
    // Invalid configuration should throw
    try {
        const invalidConfig = {
            network: 'invalid-network'
        };
        new WasmSDK(invalidConfig);
        throw new Error('Should have thrown for invalid network');
    } catch (error) {
        if (!(error instanceof WasmConfigurationError)) {
            throw new Error('Should have thrown WasmConfigurationError');
        }
    }
});

// Test 3: Configuration utilities
test('Configuration utilities work correctly', () => {
    const testnetConfig = ConfigUtils.createTestnetConfig({ 
        proofs: false 
    });
    
    if (testnetConfig.network !== 'testnet') {
        throw new Error('Testnet config not created correctly');
    }
    
    if (testnetConfig.proofs !== false) {
        throw new Error('Override not applied correctly');
    }
    
    const mainnetConfig = ConfigUtils.createMainnetConfig();
    if (mainnetConfig.network !== 'mainnet') {
        throw new Error('Mainnet config not created correctly');
    }
    
    const customConfig = ConfigUtils.createCustomEndpointConfig(
        'https://custom.endpoint.com:1443/',
        { debug: true }
    );
    
    if (!customConfig.transport.url || customConfig.debug !== true) {
        throw new Error('Custom endpoint config not created correctly');
    }
});

// Test 4: Error handling
test('Error classes work correctly', () => {
    const error = new WasmConfigurationError(
        'Test error',
        'testField',
        'testValue',
        { extra: 'context' }
    );
    
    if (error.name !== 'WasmConfigurationError') {
        throw new Error('Error name not set correctly');
    }
    
    if (error.field !== 'testField') {
        throw new Error('Error field not set correctly');
    }
    
    if (error.code !== 'WASM_CONFIG_ERROR') {
        throw new Error('Error code not set correctly');
    }
    
    const json = error.toJSON();
    if (!json.timestamp || !json.context || json.context.extra !== 'context') {
        throw new Error('Error serialization not working correctly');
    }
});

// Test 5: Resource management
test('Resource manager is properly initialized', () => {
    const sdk = new WasmSDK();
    const stats = sdk.getResourceStats();
    
    if (typeof stats.totalResources !== 'number') {
        throw new Error('Resource stats not working');
    }
    
    if (stats.totalResources !== 0) {
        throw new Error('Initial resource count should be 0');
    }
});

// Test 6: Methods exist and throw before initialization  
test('SDK methods exist but require initialization', () => {
    const sdk = new WasmSDK();
    
    // Check that methods exist
    if (typeof sdk.getIdentity !== 'function') {
        throw new Error('getIdentity method not found');
    }
    
    if (typeof sdk.getDataContract !== 'function') {
        throw new Error('getDataContract method not found');
    }
    
    if (typeof sdk.getDocuments !== 'function') {
        throw new Error('getDocuments method not found');
    }
    
    if (typeof sdk.destroy !== 'function') {
        throw new Error('destroy method not found');
    }
    
    // This test will check that methods exist
    // The initialization check will be tested in async tests
});

// Test 7: URL validation
test('URL validation works correctly', () => {
    try {
        new WasmSDK({
            transport: {
                url: 'http://insecure.com/' // HTTP not allowed
            }
        });
        throw new Error('Should have rejected HTTP URL');
    } catch (error) {
        if (!(error instanceof WasmConfigurationError)) {
            throw new Error('Should have thrown WasmConfigurationError for HTTP URL');
        }
    }
    
    try {
        new WasmSDK({
            transport: {
                url: 'not-a-url'
            }
        });
        throw new Error('Should have rejected invalid URL');
    } catch (error) {
        if (!(error instanceof WasmConfigurationError)) {
            throw new Error('Should have thrown WasmConfigurationError for invalid URL');
        }
    }
    
    // HTTPS should work
    const sdk = new WasmSDK({
        transport: {
            url: 'https://valid.endpoint.com:1443/'
        }
    });
    
    if (!sdk) {
        throw new Error('Valid HTTPS URL should work');
    }
});

// Test 8: Parameter validation (async test due to parameter validation in SDK methods)
// This will be tested in async section

// Run async tests
async function runAsyncTests() {
    // Test 9: Initialization check for API methods
    await asyncTest('API methods require initialization', async () => {
        const sdk = new WasmSDK();
        
        try {
            await sdk.getIdentity('test-id');
            throw new Error('Should have thrown for uninitialized SDK');
        } catch (error) {
            if (!error.message.includes('not initialized')) {
                throw new Error(`Wrong error message for uninitialized SDK: ${error.message}`);
            }
        }
    });
    
    // Test 10: Initialization attempt (will fail without WASM module, but should handle gracefully)
    await asyncTest('Initialization handles missing WASM module gracefully', async () => {
        const sdk = new WasmSDK();
        
        try {
            await sdk.initialize();
            // If this succeeds, we actually have the WASM module available
            console.log('    (WASM module is available - this is good!)');
        } catch (error) {
            // Expected error since we don't have the built WASM module in the test environment
            if (!error.message.includes('Failed to load WASM module') && 
                !error.message.includes('Failed to initialize WASM SDK')) {
                throw new Error(`Unexpected error during initialization: ${error.message}`);
            }
        }
    });

    // Test 10: Cleanup works
    await asyncTest('SDK cleanup works correctly', async () => {
        const sdk = new WasmSDK();
        
        // Destroy should work even without initialization
        await sdk.destroy();
        
        // Multiple destroy calls should be safe
        await sdk.destroy();
        await sdk.destroy();
    });
}

// Run all tests
console.log('Running synchronous tests...\n');

// Run async tests
await runAsyncTests();

// Summary
console.log('\n📊 Test Results:');
console.log(`Total Tests: ${testCount}`);
console.log(`Passed: ${passedCount} ✅`);
console.log(`Failed: ${failedCount} ❌`);

if (failedCount === 0) {
    console.log('\n🎉 All tests passed!');
    process.exit(0);
} else {
    console.log(`\n💥 ${failedCount} tests failed`);
    process.exit(1);
}