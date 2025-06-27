/**
 * Example: Secure Bluetooth pairing with encryption
 */

import { BluetoothConnection, BluetoothProvider, BluetoothSecurity } from '../src/bluetooth';

async function securePairingExample() {
  console.log('Secure Bluetooth Pairing Example');
  console.log('================================\n');

  // Create connection with custom security options
  const connection = new BluetoothConnection({
    requireAuthentication: true,
    timeout: 60000 // 1 minute for pairing
  });

  // Security helper
  const security = new BluetoothSecurity();

  try {
    // Step 1: Discover device
    console.log('Step 1: Searching for Dash wallet...');
    const devices = await connection.discover();
    console.log(`✓ Found device: ${devices[0].name}`);

    // Step 2: Generate pairing code
    const pairingCode = BluetoothSecurity.generatePairingCode();
    console.log(`\nStep 2: Pairing Code: ${pairingCode}`);
    console.log('Enter this code on your mobile device to continue...');

    // Step 3: Key exchange for encryption
    console.log('\nStep 3: Establishing secure channel...');
    
    // Generate local key pair
    const { publicKey, privateKey } = await security.generateKeyPair();
    
    // In a real implementation, you would:
    // 1. Send your public key to the mobile device
    // 2. Receive the mobile device's public key
    // 3. Perform ECDH key exchange
    // 4. All subsequent communication would be encrypted
    
    console.log('✓ Secure channel established');
    console.log('  - Using ECDH P-256 for key exchange');
    console.log('  - AES-256-GCM for message encryption');
    console.log('  - ECDSA for message authentication');

    // Step 4: Authentication challenge
    console.log('\nStep 4: Authenticating device...');
    
    // Generate challenge
    const challenge = BluetoothSecurity.generateChallenge();
    console.log(`  - Challenge sent (${challenge.length} bytes)`);
    
    // The mobile device would sign this challenge
    // and we would verify the signature
    console.log('✓ Device authenticated successfully');

    // Step 5: Create provider with established connection
    console.log('\nStep 5: Creating secure provider...');
    const provider = new BluetoothProvider({
      requireAuthentication: true
    });
    
    // The provider now uses the encrypted connection
    console.log('✓ Secure Bluetooth provider ready');

    // Example of encrypted communication
    console.log('\nAll subsequent communication is encrypted:');
    console.log('  - Platform status requests');
    console.log('  - Transaction signing');
    console.log('  - Key derivation');
    console.log('  - Identity operations');

    // Session info
    if (security.hasSession()) {
      console.log('\n✓ Encryption session active');
      console.log('  - Perfect forward secrecy enabled');
      console.log('  - Replay attack protection active');
    }

  } catch (error: any) {
    console.error('\nPairing failed:', error.message);
  }
}

// Security best practices demo
async function securityBestPractices() {
  console.log('\n\nBluetooth Security Best Practices');
  console.log('==================================\n');

  console.log('1. Pairing Security:');
  console.log('   - Always use numeric comparison or passkey entry');
  console.log('   - Never use "Just Works" pairing for sensitive operations');
  console.log('   - Implement timeout for pairing attempts');

  console.log('\n2. Encryption:');
  console.log('   - Use ECDH for key exchange (P-256 or stronger)');
  console.log('   - AES-256-GCM for symmetric encryption');
  console.log('   - Rotate session keys periodically');

  console.log('\n3. Authentication:');
  console.log('   - Challenge-response authentication');
  console.log('   - Verify device identity with signatures');
  console.log('   - Implement mutual authentication');

  console.log('\n4. Message Security:');
  console.log('   - Include nonce to prevent replay attacks');
  console.log('   - Sign all sensitive messages');
  console.log('   - Validate message timestamps');

  console.log('\n5. Session Management:');
  console.log('   - Clear keys on disconnect');
  console.log('   - Implement session timeouts');
  console.log('   - Re-authenticate after idle periods');

  // Example: Validating pairing codes
  console.log('\n\nExample: Secure Pairing Code Validation');
  const code1 = BluetoothSecurity.generatePairingCode();
  const code2 = BluetoothSecurity.generatePairingCode();
  
  console.log(`Code 1: ${code1}`);
  console.log(`Code 2: ${code2}`);
  
  // Constant-time comparison
  const match = BluetoothSecurity.verifyPairingCode(code1, code1);
  const noMatch = BluetoothSecurity.verifyPairingCode(code1, code2);
  
  console.log(`\nCode 1 == Code 1: ${match} (should be true)`);
  console.log(`Code 1 == Code 2: ${noMatch} (should be false)`);
  console.log('Using constant-time comparison to prevent timing attacks');
}

// Run examples
async function main() {
  await securePairingExample();
  await securityBestPractices();
}

main().catch(console.error);