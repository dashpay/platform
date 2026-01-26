import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import {
  identifier,
  identityBytesWithoutKeys,
  identifierBytes,
  balance,
  revision,
} from './mocks/Identity/index.js';
import {
  keyId, purpose, securityLevel, keyType, binaryData,
} from './mocks/PublicKey/index.js';

before(async () => {
  await initWasm();
});

describe('Identity', () => {
  describe('constructor()', () => {
    it('should create identity from identifier', async () => {
      const identity = new wasm.Identity(identifier);

      expect(identity).to.be.an.instanceof(wasm.Identity);
    });
  });

  describe('toBytes()', () => {
    it('should return identity as bytes', async () => {
      const identity = new wasm.Identity(identifier);

      expect(Array.from(identity.toBytes())).to.deep.equal(identityBytesWithoutKeys);
    });
  });

  describe('fromBytes()', () => {
    it('should recreate identity from bytes', async () => {
      const identity = new wasm.Identity(identifier);
      const newIdentity = wasm.Identity.fromBytes(identity.toBytes());

      expect(identity).to.be.an.instanceof(wasm.Identity);
      expect(newIdentity).to.be.an.instanceof(wasm.Identity);
      expect(Array.from(newIdentity.toBytes())).to.deep.equal(identityBytesWithoutKeys);
    });
  });

  describe('toJSON()', () => {
    it('should serialize identity to JSON', () => {
      const identity = new wasm.Identity(identifier);
      const identityJson = identity.toJSON();

      expect(identityJson).to.be.an('object');
    });
  });

  describe('fromJSON()', () => {
    it('should recreate identity from JSON', () => {
      const identity = new wasm.Identity(identifier);
      const identityJson = identity.toJSON();

      const restoredIdentity = wasm.Identity.fromJSON(identityJson);

      expect(Array.from(restoredIdentity.toBytes())).to.deep.equal(Array.from(identity.toBytes()));
    });
  });

  describe('toObject()', () => {
    it('should serialize identity to plain JS object', () => {
      const identity = new wasm.Identity(identifier);
      const identityObject = identity.toObject();

      // toObject returns plain JS values (Uint8Array for id, not Identifier instance)
      expect(identityObject.id.constructor.name).to.equal('Uint8Array');
      expect(identityObject.id.length).to.equal(32);
      expect(Array.isArray(identityObject.publicKeys)).to.equal(true);
      expect(identityObject.balance).to.equal(BigInt(0));
      expect(identityObject.revision).to.equal(BigInt(0));
    });
  });

  describe('fromObject()', () => {
    it('should recreate identity from object', () => {
      const identity = new wasm.Identity(identifier);
      const identityObject = identity.toObject();

      const restoredIdentity = wasm.Identity.fromObject(identityObject);

      expect(Array.from(restoredIdentity.toBytes())).to.deep.equal(Array.from(identity.toBytes()));
      expect(restoredIdentity.id.toBytes()).to.deep.equal(identity.id.toBytes());
      expect(restoredIdentity.publicKeys.length).to.equal(identity.publicKeys.length);
    });
  });

  describe('id', () => {
    it('should return id as Identifier', () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.id.toBytes()).to.deep.equal(Uint8Array.from(identifierBytes));
    });
  });

  describe('balance', () => {
    it('should return balance', () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.balance).to.deep.equal(BigInt(0));
    });

    it('should set balance', () => {
      const identity = new wasm.Identity(identifier);

      identity.balance = balance;

      expect(identity.balance).to.equal(balance);
    });
  });

  describe('revision', () => {
    it('should return revision', () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.revision).to.deep.equal(BigInt(0));
    });

    it('should set revision', () => {
      const identity = new wasm.Identity(identifier);

      identity.revision = revision;

      expect(identity.revision).to.equal(revision);
    });
  });

  describe('publicKeys', () => {
    it('should return public keys array', () => {
      const identity = new wasm.Identity(identifier);

      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const pubKey2 = new wasm.IdentityPublicKey({
        keyId: keyId + 1,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      identity.addPublicKey(pubKey);
      identity.addPublicKey(pubKey2);

      expect(identity.publicKeys.length).to.equal(2);
    });
  });

  describe('addPublicKey()', () => {
    it('should add public key to identity', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const identity = new wasm.Identity(identifier);

      identity.addPublicKey(pubKey);

      expect(identity).to.be.an.instanceof(wasm.Identity);
      expect(identity.getPublicKeyById(keyId).toBytes()).to.deep.equal(pubKey.toBytes());
    });
  });

  describe('getPublicKeyById()', () => {
    it('should return public key by id', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const identity = new wasm.Identity(identifier);
      identity.addPublicKey(pubKey);

      expect(identity.getPublicKeyById(keyId).toBytes()).to.deep.equal(pubKey.toBytes());
    });
  });
});
