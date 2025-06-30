// Example of using BLS signatures in the WASM SDK

import init, {
  // BLS functions
  generateBlsPrivateKey,
  blsPrivateKeyToPublicKey,
  blsSign,
  blsVerify,
  validateBlsPublicKey,
  getBlsSignatureSize,
  getBlsPublicKeySize,
  getBlsPrivateKeySize,
  
  // Signer classes
  WasmSigner,
  
  // Identity functions for BLS keys
  createIdentity,
  validateIdentityPublicKeys,
} from '../pkg/wasm_sdk.js';

// Initialize WASM
await init();

// Example 1: Generate and use BLS keys
async function blsKeyExample() {
  console.log('=== BLS Key Generation Example ===');
  
  // Generate a new BLS private key
  const privateKey = generateBlsPrivateKey();
  console.log('Private key size:', privateKey.length, 'bytes');
  console.log('Expected size:', getBlsPrivateKeySize(), 'bytes');
  
  // Derive the public key
  const publicKey = blsPrivateKeyToPublicKey(privateKey);
  console.log('Public key size:', publicKey.length, 'bytes');
  console.log('Expected size:', getBlsPublicKeySize(), 'bytes');
  
  // Validate the public key
  const isValid = validateBlsPublicKey(publicKey);
  console.log('Public key is valid:', isValid);
  
  return { privateKey, publicKey };
}

// Example 2: Sign and verify data with BLS
async function blsSignatureExample() {
  console.log('\n=== BLS Signature Example ===');
  
  // Generate a key pair
  const privateKey = generateBlsPrivateKey();
  const publicKey = blsPrivateKeyToPublicKey(privateKey);
  
  // Data to sign
  const message = new TextEncoder().encode('Hello, BLS signatures!');
  
  // Sign the data
  const signature = blsSign(message, privateKey);
  console.log('Signature size:', signature.length, 'bytes');
  console.log('Expected size:', getBlsSignatureSize(), 'bytes');
  
  // Verify the signature
  const isValid = blsVerify(signature, message, publicKey);
  console.log('Signature is valid:', isValid);
  
  // Try with wrong data
  const wrongMessage = new TextEncoder().encode('Wrong message');
  const isInvalid = blsVerify(signature, wrongMessage, publicKey);
  console.log('Wrong message verification (should be false):', isInvalid);
  
  return signature;
}

// Example 3: Using BLS keys with the WasmSigner
async function wasmSignerBlsExample() {
  console.log('\n=== WasmSigner with BLS Example ===');
  
  // Create a signer
  const signer = new WasmSigner();
  
  // Generate BLS key
  const privateKey = generateBlsPrivateKey();
  const publicKey = blsPrivateKeyToPublicKey(privateKey);
  
  // Add the BLS key to the signer
  const keyId = 1;
  signer.addPrivateKey(
    keyId,
    Array.from(privateKey), // Convert to array for WASM
    "BLS12_381",
    5 // VOTING purpose
  );
  
  console.log('Added BLS key with ID:', keyId);
  console.log('Signer has key:', signer.hasKey(keyId));
  console.log('Total keys in signer:', signer.getKeyCount());
  
  // Sign data using the signer
  const message = new TextEncoder().encode('Sign this with BLS');
  const signature = await signer.signData(Array.from(message), keyId);
  
  console.log('Signature created via signer, length:', signature.length);
  
  // Verify externally
  const isValid = blsVerify(new Uint8Array(signature), message, publicKey);
  console.log('External verification:', isValid);
  
  return signer;
}

// Example 4: Create an identity with BLS keys
async function identityWithBlsExample() {
  console.log('\n=== Identity with BLS Keys Example ===');
  
  // Generate keys
  const ecdsaPrivateKey = new Uint8Array(32);
  crypto.getRandomValues(ecdsaPrivateKey);
  
  const blsPrivateKey = generateBlsPrivateKey();
  const blsPublicKey = blsPrivateKeyToPublicKey(blsPrivateKey);
  
  // Create public keys for identity
  const publicKeys = [
    {
      id: 0,
      type: "ECDSA_SECP256K1",
      purpose: 0, // AUTHENTICATION
      securityLevel: 0, // MASTER
      readOnly: false,
      data: new Uint8Array(33), // Mock ECDSA public key
    },
    {
      id: 1,
      type: "BLS12_381",
      purpose: 5, // VOTING
      securityLevel: 2, // HIGH
      readOnly: false,
      data: blsPublicKey,
    }
  ];
  
  // Fill in mock ECDSA key
  crypto.getRandomValues(publicKeys[0].data);
  publicKeys[0].data[0] = 0x02; // Valid compressed key prefix
  
  // Validate the keys
  const validation = validateIdentityPublicKeys(publicKeys);
  console.log('Key validation result:', validation);
  
  return publicKeys;
}

// Example 5: BLS threshold signatures (future functionality)
async function blsThresholdExample() {
  console.log('\n=== BLS Threshold Signatures (Future) ===');
  
  // This is a placeholder for future threshold signature support
  console.log('Threshold signatures allow multiple parties to create signature shares');
  console.log('that can be combined into a single valid signature.');
  console.log('This functionality is not yet implemented but will be useful for:');
  console.log('- Multi-party computation');
  console.log('- Distributed validator systems');
  console.log('- Secure multiparty protocols');
}

// Example 6: Performance testing
async function blsPerformanceTest() {
  console.log('\n=== BLS Performance Test ===');
  
  const iterations = 100;
  const message = new TextEncoder().encode('Performance test message');
  
  // Key generation performance
  const keyGenStart = performance.now();
  for (let i = 0; i < iterations; i++) {
    generateBlsPrivateKey();
  }
  const keyGenEnd = performance.now();
  console.log(`Key generation: ${(keyGenEnd - keyGenStart) / iterations}ms per key`);
  
  // Setup for signing test
  const privateKey = generateBlsPrivateKey();
  const publicKey = blsPrivateKeyToPublicKey(privateKey);
  
  // Signing performance
  const signStart = performance.now();
  for (let i = 0; i < iterations; i++) {
    blsSign(message, privateKey);
  }
  const signEnd = performance.now();
  console.log(`Signing: ${(signEnd - signStart) / iterations}ms per signature`);
  
  // Verification performance
  const signature = blsSign(message, privateKey);
  const verifyStart = performance.now();
  for (let i = 0; i < iterations; i++) {
    blsVerify(signature, message, publicKey);
  }
  const verifyEnd = performance.now();
  console.log(`Verification: ${(verifyEnd - verifyStart) / iterations}ms per verify`);
}

// Run all examples
(async () => {
  try {
    await blsKeyExample();
    await blsSignatureExample();
    await wasmSignerBlsExample();
    await identityWithBlsExample();
    await blsThresholdExample();
    await blsPerformanceTest();
    
    console.log('\n✅ All BLS examples completed successfully!');
  } catch (error) {
    console.error('❌ Error in BLS examples:', error);
  }
})();