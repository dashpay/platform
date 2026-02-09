/**
 * Convert a hex string to Uint8Array.
 * @param {string} hex - Hex string to convert
 * @returns {Uint8Array} The bytes
 */
function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

/**
 * Requirements for wasm-sdk functional tests.
 * These IDs/contracts should exist on the target network
 * (seeded via SDK_TEST_DATA=true yarn start).
 * @returns {object} Test requirements object
 */
export function wasmFunctionalTestRequirements() {
  return {
    // Seeded via SDK_TEST_DATA=true (identity id = 32 bytes of 0x01)
    identityId: '4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi',
    // Identity 2 (32 bytes of 0x02)
    identityId2: '8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR',
    // Identity 3 (32 bytes of 0x03)
    identityId3: 'CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8',
    dpnsContractId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    dpnsDomain: {
      // The 'dash' TLD exists by default on any network
      parent: '',
      label: 'dash',
    },
    tokenContracts: [
      // Seeded token contract (contract id = 32 bytes of 0x03)
      { contractId: 'CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8', position: 0 },
      { contractId: 'CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8', position: 1 },
      { contractId: 'CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8', position: 2 },
    ],
  };
}

/**
 * Helper to get test identity keys for state transition tests.
 * Keys are generated deterministically from the seed (identity ID's first byte).
 * @param {object} sdk - The WasmSdk instance
 * @param {number} seed - Seed for deterministic key generation (1, 2, or 3 for test identities)
 * @returns {Array} Array of key objects with keyId, privateKeyHex, publicKeyHash, etc.
 */
export function getTestIdentityKeys(sdk, seed) {
  return sdk.generateTestIdentityKeys(BigInt(seed));
}

/**
 * Creates a signer and identity key for state transition tests.
 * @param {object} sdkModule - SDK module with IdentitySigner, IdentityPublicKey, PrivateKey
 * @param {number} seed - Seed for deterministic key generation
 * @param {number} keyIndex - Which key to use:
 *   - 0 = MASTER level AUTHENTICATION key (ECDSA_SECP256K1)
 *   - 1 = CRITICAL level AUTHENTICATION key (ECDSA_SECP256K1)
 *   - 2 = HIGH level AUTHENTICATION key (ECDSA_SECP256K1)
 *   - 3 = CRITICAL level TRANSFER key (ECDSA_HASH160)
 * @returns {object} Object with { signer, identityKey, keyInfo }
 */
export function createTestSignerAndKey(sdkModule, seed, keyIndex = 2) {
  const keys = sdkModule.WasmSdk.generateTestIdentityKeys(BigInt(seed));
  const keyInfo = keys[keyIndex];

  // serde_wasm_bindgen with serialize_maps_as_objects(true) returns plain objects
  const { privateKeyHex } = keyInfo;
  const { keyId } = keyInfo;
  const { publicKeyData } = keyInfo;
  const keyTypeStr = keyInfo.keyType;
  const purposeStr = keyInfo.purpose;
  const securityLevelStr = keyInfo.securityLevel;

  // Create the signer and add the private key using PrivateKey.fromHex
  const signer = new sdkModule.IdentitySigner();
  const privateKey = sdkModule.PrivateKey.fromHex(privateKeyHex, 'testnet');
  signer.addKey(privateKey);

  // Determine read_only based on key type (transfer keys are read_only)
  const readOnly = purposeStr === 'TRANSFER';

  // IdentityPublicKey constructor expects an options object with string enum values
  // keyId might be BigInt from serialization, convert to number (u32)
  const identityKey = new sdkModule.IdentityPublicKey({
    keyId: Number(keyId),
    purpose: purposeStr, // string like 'AUTHENTICATION', 'TRANSFER'
    securityLevel: securityLevelStr, // string like 'MASTER', 'CRITICAL', 'HIGH'
    keyType: keyTypeStr, // string like 'ECDSA_SECP256K1', 'ECDSA_HASH160'
    isReadOnly: readOnly,
    data: hexToBytes(publicKeyData), // Uint8Array
  });

  return { signer, identityKey, keyInfo };
}
