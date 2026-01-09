import init, * as sdk from '../../../dist/sdk.compressed.js';
import { createTestSignerAndKey } from '../fixtures/requiredTestData.mjs';

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
    it('generates deterministic keys for seed 1', () => {
      const keys = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));

      expect(keys).to.be.an('array');
      expect(keys.length).to.equal(4); // 3 authentication keys + 1 transfer key

      // serde_wasm_bindgen returns Maps, so use .get() to access properties
      // Check structure of first key
      expect(keys[0].has('keyId')).to.be.true();
      expect(keys[0].has('privateKeyHex')).to.be.true();
      expect(keys[0].has('publicKeyData')).to.be.true();
      expect(keys[0].has('keyType')).to.be.true();
      expect(keys[0].has('purpose')).to.be.true();
      expect(keys[0].has('securityLevel')).to.be.true();

      // Verify key types for authentication keys (ECDSA_SECP256K1)
      expect(keys[0].get('keyType')).to.equal('ECDSA_SECP256K1');
      expect(keys[1].get('keyType')).to.equal('ECDSA_SECP256K1');
      expect(keys[2].get('keyType')).to.equal('ECDSA_SECP256K1');
      // Key 3 is the transfer key (ECDSA_HASH160)
      expect(keys[3].get('keyType')).to.equal('ECDSA_HASH160');

      // Verify purposes
      expect(keys[0].get('purpose')).to.equal('AUTHENTICATION');
      expect(keys[1].get('purpose')).to.equal('AUTHENTICATION');
      expect(keys[2].get('purpose')).to.equal('AUTHENTICATION');
      expect(keys[3].get('purpose')).to.equal('TRANSFER');

      // Verify security levels
      expect(keys[0].get('securityLevel')).to.equal('MASTER');
      expect(keys[1].get('securityLevel')).to.equal('CRITICAL');
      expect(keys[2].get('securityLevel')).to.equal('HIGH');
      expect(keys[3].get('securityLevel')).to.equal('CRITICAL');
    });

    it('generates different keys for different seeds', () => {
      const keys1 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));
      const keys2 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(2));

      expect(keys1[0].get('privateKeyHex')).to.not.equal(keys2[0].get('privateKeyHex'));
      expect(keys1[0].get('publicKeyData')).to.not.equal(keys2[0].get('publicKeyData'));
    });

    it('generates same keys for same seed (deterministic)', () => {
      const keys1 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));
      const keys2 = sdk.WasmSdk.generateTestIdentityKeys(BigInt(1));

      expect(keys1[0].get('privateKeyHex')).to.equal(keys2[0].get('privateKeyHex'));
      expect(keys1[0].get('publicKeyData')).to.equal(keys2[0].get('publicKeyData'));
    });
  });

  describe('createTestSignerAndKey', () => {
    it('creates a signer and identity key', () => {
      const { signer, identityKey, keyInfo } = createTestSignerAndKey(sdk, 1, 2);

      expect(signer).to.exist();
      expect(identityKey).to.exist();
      expect(keyInfo).to.exist();

      // Check signer has the key
      expect(signer.keyCount).to.equal(1);

      // Check identity key properties (keyInfo is a Map)
      expect(identityKey.keyId).to.equal(keyInfo.get('keyId'));
    });
  });
});
