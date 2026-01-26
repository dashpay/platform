import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';
import { createTestSignerAndKey } from '../fixtures/requiredTestData.ts';

/**
 * Key generation tests for wasm-sdk.
 *
 * These tests verify that deterministic key generation works correctly.
 * Keys are generated using seeded RNG to match the keys created in genesis state.
 *
 * Key indices:
 * - 0: MASTER level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 1: CRITICAL level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 2: HIGH level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 3: CRITICAL level TRANSFER key (ECDSA_HASH160) - for credit transfers
 */

describe('Key Generation Tests', function describeKeyGeneration() {
  this.timeout(30000);

  before(async () => {
    await init();
  });

  describe('generateTestIdentityKeys', () => {
    it('should generate deterministic keys for seed 1', () => {
      const keys = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));

      expect(keys).to.be.an('array');
      expect(keys.length).to.equal(4); // 3 authentication keys + 1 transfer key

      // serde_wasm_bindgen with serialize_maps_as_objects(true) returns plain objects
      // Check structure of first key
      expect(keys[0].keyId).to.exist();
      expect(keys[0].privateKeyHex).to.exist();
      expect(keys[0].publicKeyData).to.exist();
      expect(keys[0].keyType).to.exist();
      expect(keys[0].purpose).to.exist();
      expect(keys[0].securityLevel).to.exist();

      // Verify key types for authentication keys (ECDSA_SECP256K1)
      expect(keys[0].keyType).to.equal('ECDSA_SECP256K1');
      expect(keys[1].keyType).to.equal('ECDSA_SECP256K1');
      expect(keys[2].keyType).to.equal('ECDSA_SECP256K1');
      // Key 3 is the transfer key (ECDSA_HASH160)
      expect(keys[3].keyType).to.equal('ECDSA_HASH160');

      // Verify purposes
      expect(keys[0].purpose).to.equal('AUTHENTICATION');
      expect(keys[1].purpose).to.equal('AUTHENTICATION');
      expect(keys[2].purpose).to.equal('AUTHENTICATION');
      expect(keys[3].purpose).to.equal('TRANSFER');

      // Verify security levels
      expect(keys[0].securityLevel).to.equal('MASTER');
      expect(keys[1].securityLevel).to.equal('CRITICAL');
      expect(keys[2].securityLevel).to.equal('HIGH');
      expect(keys[3].securityLevel).to.equal('CRITICAL');
    });

    it('should generate different keys for different seeds', () => {
      const keys1 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));
      const keys2 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(2));

      expect(keys1[0].privateKeyHex).to.not.equal(keys2[0].privateKeyHex);
      expect(keys1[0].publicKeyData).to.not.equal(keys2[0].publicKeyData);
    });

    it('should generate same keys for same seed (deterministic)', () => {
      const keys1 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));
      const keys2 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));

      expect(keys1[0].privateKeyHex).to.equal(keys2[0].privateKeyHex);
      expect(keys1[0].publicKeyData).to.equal(keys2[0].publicKeyData);
    });
  });

  describe('createTestSignerAndKey', () => {
    it('should create a signer and identity key', () => {
      const { signer, identityKey, keyInfo } = createTestSignerAndKey(sdk, 1, 2);

      expect(signer).to.exist();
      expect(identityKey).to.exist();
      expect(keyInfo).to.exist();

      // Check signer has the key
      expect(signer.keyCount).to.equal(1);

      // Check identity key properties (keyInfo is a plain object)
      // keyInfo.keyId might be BigInt from serialization, convert for comparison
      expect(identityKey.keyId).to.equal(Number(keyInfo.keyId));
    });
  });
});
