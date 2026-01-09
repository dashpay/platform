/**
 * Requirements for wasm-sdk functional tests.
 * These IDs/contracts should exist on the target network
 * (seeded via SDK_TEST_DATA=true yarn start).
 * @returns {object} Test requirements object
 */
// eslint-disable-next-line import/prefer-default-export
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

  // serde_wasm_bindgen returns Maps, so use .get() to access properties
  const privateKeyHex = keyInfo.get('privateKeyHex');
  const keyId = keyInfo.get('keyId');
  const publicKeyData = keyInfo.get('publicKeyData');
  const keyTypeStr = keyInfo.get('keyType');
  const purposeStr = keyInfo.get('purpose');
  const securityLevelStr = keyInfo.get('securityLevel');

  // Create the signer and add the private key using PrivateKey.fromHex
  const signer = new sdkModule.IdentitySigner();
  const privateKey = sdkModule.PrivateKey.fromHex(privateKeyHex, 'testnet');
  signer.addKey(privateKey);

  // Key types: ECDSA_SECP256K1 = 0, BLS12_381 = 1, ECDSA_HASH160 = 2, BIP13_SCRIPT_HASH = 3
  const keyTypeMap = {
    ECDSA_SECP256K1: 0,
    BLS12_381: 1,
    ECDSA_HASH160: 2,
    BIP13_SCRIPT_HASH: 3,
  };
  const keyType = keyTypeMap[keyTypeStr] ?? 0;

  // Purposes: AUTHENTICATION = 0, ENCRYPTION = 1, DECRYPTION = 2, TRANSFER = 3, ...
  const purposeMap = {
    AUTHENTICATION: 0,
    ENCRYPTION: 1,
    DECRYPTION: 2,
    TRANSFER: 3,
    SYSTEM: 4,
    VOTING: 5,
    OWNER: 6,
  };
  const purpose = purposeMap[purposeStr] ?? 0;

  // Security levels: MASTER = 0, CRITICAL = 1, HIGH = 2, MEDIUM = 3
  const securityLevelMap = {
    MASTER: 0,
    CRITICAL: 1,
    HIGH: 2,
    MEDIUM: 3,
  };
  const securityLevel = securityLevelMap[securityLevelStr] ?? keyIndex;

  // Determine read_only based on key type (transfer keys are read_only)
  const readOnly = purposeStr === 'TRANSFER';

  const identityKey = new sdkModule.IdentityPublicKey(
    keyId,
    purpose,
    securityLevel,
    keyType,
    readOnly,
    publicKeyData, // binary_data as hex (33 bytes for ECDSA_SECP256K1, 20 bytes for ECDSA_HASH160)
    undefined, // disabled_at
    undefined, // contract_bounds
  );

  return { signer, identityKey, keyInfo };
}
