import getWasm from './helpers/wasm.js';
import { identifier, identifierBytes } from './mocks/Identity/index.js';
import {
  keyId, purpose, securityLevel, keyType, binaryData,
} from './mocks/PublicKey/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('PartialIdentity', () => {
  describe('constructor', () => {
    it('should create PartialIdentity with minimal options', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      expect(partialIdentity.__wbg_ptr).to.not.equal(0);
      expect(partialIdentity.id.toBase58()).to.equal(identifier);
    });

    it('should create PartialIdentity with all options', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: { [keyId]: pubKey },
        balance: BigInt(1000),
        revision: BigInt(5),
        notFoundPublicKeys: [10, 20],
      });

      expect(partialIdentity.__wbg_ptr).to.not.equal(0);
      expect(partialIdentity.id.toBase58()).to.equal(identifier);
      expect(partialIdentity.balance).to.equal(BigInt(1000));
      expect(partialIdentity.revision).to.equal(BigInt(5));
      expect(Array.from(partialIdentity.notFoundPublicKeys)).to.deep.equal([10, 20]);
    });
  });

  describe('toJSON', () => {
    it('should serialize to JSON with minimal data', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      const json = partialIdentity.toJSON();

      expect(json.id).to.equal(identifier);
      expect(json.loadedPublicKeys).to.deep.equal({});
      // JSON uses null for missing optional fields (JSON standard)
      expect(json.balance).to.equal(null);
      expect(json.revision).to.equal(null);
      expect(json.notFoundPublicKeys).to.deep.equal([]);
    });

    it('should serialize to JSON with all data', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: { [keyId]: pubKey },
        balance: BigInt(1000),
        revision: BigInt(5),
        notFoundPublicKeys: [10, 20],
      });

      const json = partialIdentity.toJSON();

      expect(json.id).to.equal(identifier);
      expect(Object.keys(json.loadedPublicKeys)).to.deep.equal([String(keyId)]);
      expect(json.loadedPublicKeys[String(keyId)]).to.be.an('object');
      // IdentityPublicKey JSON uses 'id' field not 'keyId'
      expect(json.loadedPublicKeys[String(keyId)].id).to.equal(keyId);
      expect(json.balance).to.equal(1000);
      expect(json.revision).to.equal(5);
      expect(json.notFoundPublicKeys).to.deep.equal([10, 20]);
    });
  });

  describe('toObject', () => {
    it('should serialize to object with minimal data', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      const obj = partialIdentity.toObject();

      // toObject returns Uint8Array for id (consistent with other toObject implementations)
      expect(obj.id.constructor.name).to.equal('Uint8Array');
      expect(Array.from(obj.id)).to.deep.equal(identifierBytes);
      expect(obj.loadedPublicKeys).to.deep.equal({});
      expect(obj.balance).to.equal(undefined);
      expect(obj.revision).to.equal(undefined);
      expect(Array.from(obj.notFoundPublicKeys)).to.deep.equal([]);
    });

    it('should serialize to object with all data', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: { [keyId]: pubKey },
        balance: BigInt(1000),
        revision: BigInt(5),
        notFoundPublicKeys: [10, 20],
      });

      const obj = partialIdentity.toObject();

      // toObject returns Uint8Array for id
      expect(obj.id.constructor.name).to.equal('Uint8Array');
      expect(Array.from(obj.id)).to.deep.equal(identifierBytes);
      expect(Object.keys(obj.loadedPublicKeys)).to.deep.equal([String(keyId)]);
      // loadedPublicKeys values should be plain objects (from toObject), not WASM instances
      expect(obj.loadedPublicKeys[String(keyId)]).to.be.an('object');
      expect(obj.loadedPublicKeys[String(keyId)].id).to.equal(keyId);
      expect(obj.balance).to.equal(BigInt(1000));
      expect(obj.revision).to.equal(BigInt(5));
      expect(Array.from(obj.notFoundPublicKeys)).to.deep.equal([10, 20]);
    });
  });

  describe('getters', () => {
    it('should get id', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      expect(partialIdentity.id.toBase58()).to.equal(identifier);
    });

    it('should get loadedPublicKeys', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: { [keyId]: pubKey },
      });

      const keys = partialIdentity.loadedPublicKeys;
      expect(Object.keys(keys)).to.deep.equal([String(keyId)]);
      expect(keys[String(keyId)].__type).to.equal('IdentityPublicKey');
    });

    it('should get balance', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
        balance: BigInt(500),
      });

      expect(partialIdentity.balance).to.equal(BigInt(500));
    });

    it('should get revision', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
        revision: BigInt(10),
      });

      expect(partialIdentity.revision).to.equal(BigInt(10));
    });

    it('should get notFoundPublicKeys', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
        notFoundPublicKeys: [5, 15, 25],
      });

      expect(Array.from(partialIdentity.notFoundPublicKeys)).to.deep.equal([5, 15, 25]);
    });
  });

  describe('setters', () => {
    it('should set id', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      const newId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
      partialIdentity.id = newId;

      expect(partialIdentity.id.toBase58()).to.equal(newId);
    });

    it('should set balance', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      partialIdentity.balance = BigInt(999);

      expect(partialIdentity.balance).to.equal(BigInt(999));
    });

    it('should set revision', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      partialIdentity.revision = BigInt(42);

      expect(partialIdentity.revision).to.equal(BigInt(42));
    });

    it('should set notFoundPublicKeys', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      partialIdentity.notFoundPublicKeys = [100, 200];

      expect(Array.from(partialIdentity.notFoundPublicKeys)).to.deep.equal([100, 200]);
    });
  });

  describe('fromObject', () => {
    it('should deserialize from object with minimal data', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      const obj = partialIdentity.toObject();
      const restored = wasm.PartialIdentity.fromObject(obj);

      expect(restored.id.toBase58()).to.equal(identifier);
      expect(restored.loadedPublicKeys).to.deep.equal({});
      // Getters return undefined for missing optional values (Option<T> -> undefined)
      expect(restored.balance).to.equal(undefined);
      expect(restored.revision).to.equal(undefined);
      expect(Array.from(restored.notFoundPublicKeys)).to.deep.equal([]);
    });

    it('should deserialize from object with all data', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: { [keyId]: pubKey },
        balance: BigInt(1000),
        revision: BigInt(5),
        notFoundPublicKeys: [10, 20],
      });

      const obj = partialIdentity.toObject();
      const restored = wasm.PartialIdentity.fromObject(obj);

      expect(restored.id.toBase58()).to.equal(identifier);
      expect(Object.keys(restored.loadedPublicKeys)).to.deep.equal([String(keyId)]);
      expect(restored.loadedPublicKeys[String(keyId)].__type).to.equal('IdentityPublicKey');
      expect(restored.balance).to.equal(BigInt(1000));
      expect(restored.revision).to.equal(BigInt(5));
      expect(Array.from(restored.notFoundPublicKeys)).to.deep.equal([10, 20]);
    });
  });

  describe('fromJSON', () => {
    it('should deserialize from JSON with minimal data', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      const json = partialIdentity.toJSON();
      const restored = wasm.PartialIdentity.fromJSON(json);

      expect(restored.id.toBase58()).to.equal(identifier);
      expect(restored.loadedPublicKeys).to.deep.equal({});
      // Getters return undefined for missing optional values (Option<T> -> undefined)
      expect(restored.balance).to.equal(undefined);
      expect(restored.revision).to.equal(undefined);
      expect(Array.from(restored.notFoundPublicKeys)).to.deep.equal([]);
    });

    it('should deserialize from JSON with all data', () => {
      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: { [keyId]: pubKey },
        balance: BigInt(1000),
        revision: BigInt(5),
        notFoundPublicKeys: [10, 20],
      });

      const json = partialIdentity.toJSON();
      const restored = wasm.PartialIdentity.fromJSON(json);

      expect(restored.id.toBase58()).to.equal(identifier);
      expect(Object.keys(restored.loadedPublicKeys)).to.deep.equal([String(keyId)]);
      expect(restored.loadedPublicKeys[String(keyId)].__type).to.equal('IdentityPublicKey');
      // Note: balance/revision come back as numbers in JSON (not BigInt)
      expect(restored.balance).to.equal(BigInt(1000));
      expect(restored.revision).to.equal(BigInt(5));
      expect(Array.from(restored.notFoundPublicKeys)).to.deep.equal([10, 20]);
    });
  });
});
