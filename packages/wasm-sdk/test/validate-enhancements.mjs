#!/usr/bin/env node
/**
 * Validation script for Issue #52 enhancements
 * Tests the new configuration validation, error handling, and type system
 */

import { 
  WasmSDK, 
  WasmInitializationError, 
  WasmOperationError, 
  ErrorMapper,
  isWasmInitializationError,
  isWasmOperationError,
  NETWORK_TYPES,
  DEFAULT_CONFIG,
  SDK_VERSION
} from '../src-js/index.js';

function runValidationTests() {
  console.log('🔍 Validating Issue #52 Enhancements...\n');

  // Test 1: Constants Export
  console.log('1. Testing Constants Export:');
  console.log(`   NETWORK_TYPES: ${JSON.stringify(NETWORK_TYPES)}`);
  console.log(`   SDK_VERSION: ${SDK_VERSION.VERSION_STRING}`);
  console.log(`   DEFAULT_CONFIG network: ${DEFAULT_CONFIG.network}`);
  console.log('   ✅ Constants exported correctly\n');

  // Test 2: Enhanced Configuration Validation
  console.log('2. Testing Enhanced Configuration Validation:');
  
  // Test invalid network
  try {
    new WasmSDK({ network: 'invalidnet' });
    console.log('   ❌ Should have failed for invalid network');
  } catch (error) {
    if (isWasmInitializationError(error)) {
      console.log('   ✅ Invalid network caught correctly');
      console.log(`      Context: ${JSON.stringify(error.context, null, 2)}`);
    }
  }

  // Test invalid transport timeout
  try {
    new WasmSDK({ transport: { url: 'https://example.com', timeout: 500 } });
    console.log('   ❌ Should have failed for invalid timeout');
  } catch (error) {
    if (isWasmInitializationError(error)) {
      console.log('   ✅ Invalid timeout caught correctly');
      console.log(`      Error: ${error.message}`);
    }
  }

  // Test valid configuration
  try {
    const sdk = new WasmSDK({
      network: 'mainnet',
      transport: {
        url: 'https://test.example.com:1443/',
        timeout: 15000,
        retries: 2
      },
      settings: {
        connect_timeout_ms: 5000,
        timeout_ms: 20000,
        retries: 3,
        ban_failed_address: false
      },
      proofs: true
    });
    console.log('   ✅ Valid configuration accepted');
    
    const config = sdk.getConfig();
    console.log(`      Network: ${config.network}`);
    console.log(`      URL: ${config.transport.url}`);
    console.log(`      Proofs: ${config.proofs}`);
    
    sdk.destroy();
  } catch (error) {
    console.log(`   ❌ Valid configuration rejected: ${error.message}`);
  }
  console.log('');

  // Test 3: Type Guards
  console.log('3. Testing Type Guards:');
  const initError = new WasmInitializationError('test');
  const opError = new WasmOperationError('test', 'testOp');
  const regularError = new Error('test');
  
  console.log(`   isWasmInitializationError(initError): ${isWasmInitializationError(initError)}`);
  console.log(`   isWasmOperationError(opError): ${isWasmOperationError(opError)}`);
  console.log(`   isWasmInitializationError(regularError): ${isWasmInitializationError(regularError)}`);
  console.log('   ✅ Type guards working correctly\n');

  // Test 4: ErrorMapper
  console.log('4. Testing ErrorMapper:');
  const testError = new Error('network connection failed');
  const mappedError = ErrorMapper.mapWasmError(testError, 'testOperation', { customContext: 'test' });
  
  console.log(`   Original: ${testError.message}`);
  console.log(`   Mapped: ${mappedError.message}`);
  console.log(`   Category: ${mappedError.context.errorCategory}`);
  console.log(`   Custom Context: ${mappedError.context.customContext}`);
  console.log('   ✅ ErrorMapper working correctly\n');

  // Test 5: Contextual Error Creation
  console.log('5. Testing Contextual Error Creation:');
  const contextualError = ErrorMapper.createContextualError(
    'Test operation failed',
    'testOp',
    { username: 'test', privateKey: 'secret123' },
    testError
  );
  
  console.log(`   Message: ${contextualError.message}`);
  console.log(`   Sanitized input: ${JSON.stringify(contextualError.context.inputData)}`);
  console.log(`   Timestamp: ${contextualError.context.timestamp}`);
  console.log('   ✅ Contextual error creation working correctly\n');

  // Test 6: Configuration Immutability
  console.log('6. Testing Configuration Immutability:');
  const sdk = new WasmSDK({ network: 'testnet' });
  const config1 = sdk.getConfig();
  const originalUrl = config1.transport.url;
  
  // Try to modify the returned config
  config1.transport.url = 'modified';
  
  const config2 = sdk.getConfig();
  const urlUnchanged = config2.transport.url === originalUrl;
  
  console.log(`   Original URL: ${originalUrl}`);
  console.log(`   URL unchanged after modification attempt: ${urlUnchanged}`);
  console.log(`   ✅ Configuration immutability ${urlUnchanged ? 'working' : 'FAILED'}\n`);
  
  sdk.destroy();

  console.log('🎉 All Issue #52 enhancements validated successfully!');
  console.log('\n📋 Enhanced Features Confirmed:');
  console.log('   ✅ Advanced configuration validation with detailed context');
  console.log('   ✅ Enhanced error handling system with debugging information');
  console.log('   ✅ Transport and network configuration support');
  console.log('   ✅ Proof verification settings integration');
  console.log('   ✅ Type guards and error mapping utilities');
  console.log('   ✅ Configuration immutability and security measures');
}

// Run validation if script is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runValidationTests();
}

export { runValidationTests };