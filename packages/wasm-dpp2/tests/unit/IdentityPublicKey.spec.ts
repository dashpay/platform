import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { wif } from './mocks/PrivateKey/index.js';
import {
  keyId,
  purpose,
  securityLevel,
  keyType,
  binaryData,
  securityLevelSet,
  keyIdSet,
  purposeSet,
  keyTypeSet,
  binaryDataSet,
} from './mocks/PublicKey/index.js';
import { toHexString } from './utils/hex.js';

before(async () => {
  await initWasm();
});

describe('IdentityPublicKey', () => {
  describe('constructor()', () => {
    it('should create public key from values with type ECDSA_SECP256K1', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      expect(pubKey).to.be.an.instanceof(wasm.IdentityPublicKey);
    });
  });

  describe('toBytes()', () => {
    it('should serialize public key to bytes', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const bytes = pubKey.toBytes();

      expect(bytes.length).to.be.greaterThan(0);
    });
  });

  describe('fromBytes()', () => {
    it('should deserialize public key from bytes', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const bytes = pubKey.toBytes();
      const newPubKey = wasm.IdentityPublicKey.fromBytes(Array.from(bytes));

      expect(pubKey).to.be.an.instanceof(wasm.IdentityPublicKey);
      expect(newPubKey).to.be.an.instanceof(wasm.IdentityPublicKey);

      expect(pubKey.keyId).to.equal(newPubKey.keyId);
      expect(pubKey.purpose).to.equal(newPubKey.purpose);
      expect(pubKey.securityLevel).to.equal(newPubKey.securityLevel);
      expect(pubKey.keyType).to.equal(newPubKey.keyType);
      expect(pubKey.isReadOnly).to.equal(newPubKey.isReadOnly);
      expect(pubKey.data).to.equal(newPubKey.data);

      expect(pubKey.toBytes()).to.deep.equal(newPubKey.toBytes());
    });
  });

  describe('getPublicKeyHash()', () => {
    it('should return hash of public key', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const hash = pubKey.getPublicKeyHash();

      expect(hash).to.deep.equal(
        toHexString([
          211, 114, 240, 150, 37, 159, 114, 104, 110, 24, 102, 61, 125, 181, 248, 98, 52, 221, 111,
          85,
        ]),
      );
    });
  });

  describe('keyId', () => {
    it('should return keyId', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      expect(pubKey.keyId).to.equal(keyId);
    });

    it('should set keyId', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      pubKey.keyId = keyIdSet;

      expect(pubKey.keyId).to.equal(keyIdSet);
    });
  });

  describe('purpose', () => {
    it('should return purpose', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      expect(pubKey.purpose).to.equal('AUTHENTICATION');
    });

    it('should set purpose', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      pubKey.purpose = purposeSet;

      expect(pubKey.purpose).to.equal('ENCRYPTION');
    });
  });

  describe('securityLevel', () => {
    it('should return securityLevel', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      expect(pubKey.securityLevel).to.equal('CRITICAL');
    });

    it('should set securityLevel', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      pubKey.securityLevel = securityLevelSet;

      expect(pubKey.securityLevel).to.equal('HIGH');
    });
  });

  describe('keyType', () => {
    it('should return keyType', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      expect(pubKey.keyType).to.equal('ECDSA_SECP256K1');
    });

    it('should set keyType', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      pubKey.keyType = keyTypeSet;

      expect(pubKey.keyType).to.equal('ECDSA_HASH160');
    });
  });

  describe('isReadOnly', () => {
    it('should return isReadOnly', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      expect(pubKey.isReadOnly).to.equal(false);
    });

    it('should set isReadOnly', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      pubKey.isReadOnly = true;

      expect(pubKey.isReadOnly).to.equal(true);
    });
  });

  describe('data', () => {
    it('should return data', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      expect(pubKey.data).to.equal(binaryData);
    });

    it('should set data', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      pubKey.data = binaryDataSet;

      expect(pubKey.data).to.equal(binaryDataSet);
    });
  });

  describe('validatePrivateKey()', () => {
    it('should validate private key against public key', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const privateKey = wasm.PrivateKey.fromWIF(wif);

      expect(pubKey.validatePrivateKey(privateKey.toBytes(), wasm.Network.Mainnet)).to.equal(false);
    });
  });
});
