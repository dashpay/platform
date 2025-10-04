import getWasm from './helpers/wasm.js';
import {
  identifier, identityBytesWithoutKeys, identifierBytes, balance, revision,
} from './mocks/Identity/index.js';
import {
  keyId, purpose, securityLevel, keyType, binaryData,
} from './mocks/PublicKey/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('Identity', () => {
  describe('serialization / deserialization', () => {
    it('should generate identity from identifier', async () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.__wbg_ptr).to.not.equal(0);
    });

    it('should generate identity from identifier and return bytes', async () => {
      const identity = new wasm.Identity(identifier);

      expect(Array.from(identity.bytes())).to.deep.equal(identityBytesWithoutKeys);

      const newIdentity = wasm.Identity.fromBytes(identity.bytes());

      expect(identity.__wbg_ptr).to.not.equal(0);
      expect(newIdentity.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should get id buffer', () => {
      const identity = new wasm.Identity(identifier);

      expect(identity.id.bytes()).to.deep.equal(Uint8Array.from(identifierBytes));
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

      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      const pubKey2 = new wasm.IdentityPublicKey(
        keyId + 1,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      identity.addPublicKey(pubKey);
      identity.addPublicKey(pubKey2);

      expect(identity.getPublicKeys().length).to.equal(2);
    });
  });

  describe('setters', () => {
    it('should allows to set public key', () => {
      const pubKey = new wasm.IdentityPublicKey(
        keyId,
        purpose,
        securityLevel,
        keyType,
        false,
        binaryData,
      );

      const identity = new wasm.Identity(identifier);

      identity.addPublicKey(pubKey);

      expect(identity.__wbg_ptr).to.not.equal(0);

      expect(identity.getPublicKeyById(keyId).bytes()).to.deep.equal(pubKey.bytes());
    });

    it('should allows to set balance', () => {
      const identity = new wasm.Identity(identifier);

      identity.balance = balance;

      expect(identity.balance).to.equal(balance);
    });

    it('should allows to set revision', () => {
      const identity = new wasm.Identity(identifier);

      identity.revision = revision;

      expect(identity.revision).to.equal(revision);
    });
  });
});
