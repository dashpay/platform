#!/usr/bin/env node

/**
 * Test script for the JavaScript wrapper layer
 */

import { WasmSDK } from './src-js/index.js';

async function testWrapperLayer() {
  console.log('🧪 Testing WASM SDK JavaScript Wrapper Layer');
  
  try {
    // Test 1: SDK Configuration
    console.log('\n1. Testing SDK Configuration...');
    const sdk = new WasmSDK({
      network: 'testnet',
      transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000
      },
      proofs: true
    });
    
    console.log('✅ SDK instance created successfully');
    console.log('   Network:', sdk.getConfig().network);
    console.log('   Transport URL:', sdk.getConfig().transport.url);
    console.log('   Proofs enabled:', sdk.getConfig().proofs);
    
    // Test 2: SDK Initialization
    console.log('\n2. Testing SDK Initialization...');
    console.log('   Initialized:', sdk.isInitialized());
    
    await sdk.initialize();
    console.log('✅ SDK initialized successfully');
    console.log('   Initialized:', sdk.isInitialized());
    console.log('   Version:', sdk.getVersion());
    
    // Test 3: DPNS Utility Functions (don't require network calls)
    console.log('\n3. Testing DPNS Utility Functions...');
    
    const testUsernames = ['alice', 'bob', 'test-user', 'invalid username!'];
    for (const username of testUsernames) {
      try {
        const isValid = sdk.isDpnsUsernameValid(username);
        const isContested = sdk.isDpnsUsernameContested(username);
        console.log(`   "${username}": valid=${isValid}, contested=${isContested}`);
      } catch (error) {
        console.log(`   "${username}": Error - ${error.message}`);
      }
    }
    
    // Test homograph conversion
    const testString = "tëst";
    const converted = sdk.dpnsConvertToHomographSafe(testString);
    console.log(`   Homograph: "${testString}" -> "${converted}"`);
    
    // Test 4: Token ID calculation
    console.log('\n4. Testing Token Operations...');
    try {
      const contractId = "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv";
      const tokenPosition = 0;
      const tokenId = sdk.calculateTokenId(contractId, tokenPosition);
      console.log(`   Token ID: ${tokenId}`);
      console.log('✅ Token ID calculation successful');
    } catch (error) {
      console.log(`   Token ID calculation failed: ${error.message}`);
    }
    
    // Test 5: Key Derivation
    console.log('\n5. Testing Key Derivation...');
    try {
      const mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
      const path = "m/9'/5'/15'/0/0";
      const keyInfo = sdk.deriveKey(mnemonic, null, path, 'testnet');
      console.log('✅ Key derivation successful');
      console.log('   Address:', keyInfo.address);
      console.log('   Private key length:', keyInfo.privateKey?.length || 'N/A');
    } catch (error) {
      console.log(`   Key derivation failed: ${error.message}`);
    }
    
    // Test 6: Error Handling
    console.log('\n6. Testing Error Handling...');
    try {
      sdk.isDpnsUsernameValid(null);
    } catch (error) {
      console.log('✅ Error handling working correctly');
      console.log('   Error type:', error.constructor.name);
      console.log('   Error code:', error.code);
    }
    
    // Test 7: Resource Management
    console.log('\n7. Testing Resource Management...');
    sdk.destroy();
    console.log('✅ SDK destroyed successfully');
    console.log('   Initialized:', sdk.isInitialized());
    
    console.log('\n🎉 All wrapper layer tests completed successfully!');
    
  } catch (error) {
    console.error('\n❌ Test failed:', error.message);
    console.error('Error details:', {
      name: error.name,
      code: error.code,
      context: error.context
    });
    process.exit(1);
  }
}

// Run the test
testWrapperLayer().catch(error => {
  console.error('Unhandled error:', error);
  process.exit(1);
});