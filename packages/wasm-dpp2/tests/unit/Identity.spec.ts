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
  describe('serialization / deserialization', () => {
    it('should generate identity from identifier', async () => {
      const identity = new wasm.Identity(identifier);

      expect(identity).to.be.an.instanceof(wasm.Identity);
    });

    it('should generate identity from identifier and return bytes', async () => {
      const identity = new wasm.Identity(identifier);

      expect(Array.from(identity.toBytes())).to.deep.equal(identityBytesWithoutKeys);

      const newIdentity = wasm.Identity.fromBytes(identity.toBytes());

      expect(identity).to.be.an.instanceof(wasm.Identity);
      expect(newIdentity).to.be.an.instanceof(wasm.Identity);
    });

    it('should recreate identity from JSON output', () => {
      const identity = new wasm.Identity(identifier);
      const identityJson = identity.toJSON();

      const restoredIdentity = wasm.Identity.fromJSON(identityJson);

      expect(Array.from(restoredIdentity.toBytes())).to.deep.equal(Array.from(identity.toBytes()));
    });

    it('should recreate identity from object output', () => {
      const identity = new wasm.Identity(identifier);
      const identityObject = identity.toObject();

      // toObject returns plain JS values (Uint8Array for id, not Identifier instance)
      expect(identityObject.id.constructor.name).to.equal('Uint8Array');
      expect(identityObject.id.length).to.equal(32);
      expect(Array.isArray(identityObject.publicKeys)).to.equal(true);
      expect(identityObject.balance).to.equal(BigInt(0));
      expect(identityObject.revision).to.equal(BigInt(0));

      const restoredIdentity = wasm.Identity.fromObject(identityObject);

      expect(Array.from(restoredIdentity.toBytes())).to.deep.equal(Array.from(identity.toBytes()));
      expect(restoredIdentity.id.toBytes()).to.deep.equal(identity.id.toBytes());
      expect(restoredIdentity.publicKeys.length).to.equal(identity.publicKeys.length);
    });
  });

  describe('getters', () => {
    it('should get id buffer', () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.id.toBytes()).to.deep.equal(Uint8Array.from(identifierBytes));
    });

    it('should get balance', () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.balance).to.deep.equal(BigInt(0));
    });

    it('should get revision', () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.revision).to.deep.equal(BigInt(0));
    });

    it('should get public keys', () => {
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

  describe('setters', () => {
    it('should allow to set public key', () => {
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

    it('should allow to set balance', () => {
      const identity = new wasm.Identity(identifier);

      identity.balance = balance;

      expect(identity.balance).to.equal(balance);
    });

    it('should allow to set revision', () => {
      const identity = new wasm.Identity(identifier);

      identity.revision = revision;

      expect(identity.revision).to.equal(revision);
    });
  });
});
