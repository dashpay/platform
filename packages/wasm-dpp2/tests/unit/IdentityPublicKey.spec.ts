import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { wif } from './mocks/PrivateKey/index.js';
import {
  keyId,
  purpose,
  securityLevel,
  keyType,
  binaryData,
  binaryDataHex,
  securityLevelSet,
  keyIdSet,
  purposeSet,
  keyTypeSet,
  binaryDataSetHex,
} from './mocks/PublicKey/index.js';
import { toHexString } from './utils/hex.ts';

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

      expect(pubKey.data).to.equal(binaryDataHex);
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

      pubKey.data = binaryDataSetHex;

      expect(pubKey.data).to.equal(binaryDataSetHex);
    });
  });

  describe('toJSON()', () => {
    it('should serialize to JSON with expected fixture values', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const json = pubKey.toJSON();

      expect(json.id).to.equal(keyId);
      expect(json.purpose).to.equal(0); // AUTHENTICATION
      expect(json.securityLevel).to.equal(1); // CRITICAL
      expect(json.contractBounds).to.be.null();
      expect(json.type).to.equal(0); // ECDSA_SECP256K1
      expect(json.readOnly).to.equal(false);
      expect(Array.from(json.data)).to.deep.equal(Array.from(binaryData));
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize from JSON and verify all fields via getters', () => {
      const jsonFixture = {
        $formatVersion: '0',
        id: keyId,
        purpose: 0,
        securityLevel: 1,
        contractBounds: null,
        type: 0,
        readOnly: false,
        data: 'A2o5QxLkDoHZKP3iveeIAHDk+pwdHZsWjacH6kaK+itI',
      };

      const pubKey = wasm.IdentityPublicKey.fromJSON(jsonFixture);

      expect(pubKey).to.be.an.instanceof(wasm.IdentityPublicKey);
      expect(pubKey.keyId).to.equal(keyId);
      expect(pubKey.purpose).to.equal('AUTHENTICATION');
      expect(pubKey.securityLevel).to.equal('CRITICAL');
      expect(pubKey.keyType).to.equal('ECDSA_SECP256K1');
      expect(pubKey.isReadOnly).to.equal(false);
      expect(pubKey.data).to.equal(binaryDataHex);
    });
  });

  describe('toObject()', () => {
    it('should serialize to Object with expected fixture values', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const obj = pubKey.toObject();

      expect(obj.id).to.equal(keyId);
      expect(obj.purpose).to.equal(0); // AUTHENTICATION
      expect(obj.securityLevel).to.equal(1); // CRITICAL
      expect(obj.type).to.equal(0); // ECDSA_SECP256K1
      expect(obj.readOnly).to.equal(false);
      expect(obj.data).to.be.an.instanceof(Uint8Array);
      expect(obj.data).to.deep.equal(binaryData);
    });
  });

  describe('fromObject()', () => {
    it('should deserialize from Object and verify all fields via getters', () => {
      const objectFixture = {
        $formatVersion: '0',
        id: keyId,
        purpose: 0,
        securityLevel: 1,
        type: 0,
        readOnly: false,
        data: binaryData,
      };

      const pubKey = wasm.IdentityPublicKey.fromObject(objectFixture);

      expect(pubKey).to.be.an.instanceof(wasm.IdentityPublicKey);
      expect(pubKey.keyId).to.equal(keyId);
      expect(pubKey.purpose).to.equal('AUTHENTICATION');
      expect(pubKey.securityLevel).to.equal('CRITICAL');
      expect(pubKey.keyType).to.equal('ECDSA_SECP256K1');
      expect(pubKey.isReadOnly).to.equal(false);
      expect(pubKey.data).to.equal(binaryDataHex);
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
