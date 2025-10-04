import getWasm from './helpers/wasm.js';
import { wif } from './mocks/PrivateKey/index.js';
import {
  keyId, purpose, securityLevel, keyType, binaryData, securityLevelSet, keyIdSet, purposeSet, keyTypeSet, binaryDataSet,
} from './mocks/PublicKey/index.js';
import { toHexString } from './utils/hex.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('PublicKey', () => {
  describe('serialization / deserialization', () => {
    it('should generate public key from values with type ECDSA_SECP256K1', () => {
      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      expect(pubKey.__wbg_ptr).to.not.equal(0);
    });

    it('should generate public key from values with type ECDSA_SECP256K1 and generate new from self bytes', () => {
      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      const bytes = pubKey.bytes();

      const newPubKey = wasm.IdentityPublicKey.fromBytes(Array.from(bytes));

      expect(pubKey.__wbg_ptr).to.not.equal(0);
      expect(newPubKey.__wbg_ptr).to.not.equal(0);

      expect(pubKey.keyId).to.equal(newPubKey.keyId);
      expect(pubKey.purpose).to.equal(newPubKey.purpose);
      expect(pubKey.securityLevel).to.equal(newPubKey.securityLevel);
      expect(pubKey.keyType).to.equal(newPubKey.keyType);
      expect(pubKey.readOnly).to.equal(newPubKey.readOnly);
      expect(pubKey.data).to.equal(newPubKey.data);

      expect(pubKey.bytes()).to.deep.equal(newPubKey.bytes());

      expect(pubKey.__wbg_ptr).to.not.equal(0);
      expect(newPubKey.__wbg_ptr).to.not.equal(0);
    });

    it('should return hash of key', () => {
      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      const hash = pubKey.getPublicKeyHash();

      expect(hash).to.deep.equal(toHexString([211, 114, 240, 150, 37, 159, 114, 104, 110, 24, 102, 61, 125, 181, 248, 98, 52, 221, 111, 85]));
    });
  });
  describe('getters', () => {
    it('should generate public key from values with type ECDSA_SECP256K1 and return all fields', () => {
      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      expect(pubKey.keyId).to.equal(keyId);
      expect(pubKey.purpose).to.equal('AUTHENTICATION');
      expect(pubKey.securityLevel).to.equal('CRITICAL');
      expect(pubKey.keyType).to.equal('ECDSA_SECP256K1');
      expect(pubKey.readOnly).to.equal(false);
      expect(pubKey.data).to.equal(binaryData);
    });

    it('should allow to validate private key', () => {
      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      const privateKey = wasm.PrivateKey.fromWIF(wif);

      expect(pubKey.validatePrivateKey(privateKey.bytes(), wasm.Network.Mainnet)).to.equal(false);
    });
  });

  describe('setters', () => {
    it('should generate public key from values with type ECDSA_SECP256K1 and return all fields and set another fields', () => {
      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      pubKey.keyId = keyIdSet;
      pubKey.purpose = purposeSet;
      pubKey.securityLevel = securityLevelSet;
      pubKey.keyType = keyTypeSet;
      pubKey.readOnly = true;
      pubKey.data = binaryDataSet;

      expect(pubKey.keyId).to.equal(keyIdSet);
      expect(pubKey.purpose).to.equal('ENCRYPTION');
      expect(pubKey.securityLevel).to.equal('HIGH');
      expect(pubKey.keyType).to.equal('ECDSA_HASH160');
      expect(pubKey.readOnly).to.equal(true);
      expect(pubKey.data).to.equal(binaryDataSet);
    });
  });
});
