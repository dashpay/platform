import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { identifier, identifierBytes } from './mocks/Identity/index.js';
import {
  keyId, purpose, securityLevel, keyType, binaryData,
} from './mocks/PublicKey/index.js';

before(async () => {
  await initWasm();
});

describe('PartialIdentity', () => {
  describe('constructor()', () => {
    it('should create PartialIdentity with minimal options', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      expect(partialIdentity).to.be.an.instanceof(wasm.PartialIdentity);
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

      expect(partialIdentity).to.be.an.instanceof(wasm.PartialIdentity);
      expect(partialIdentity.id.toBase58()).to.equal(identifier);
      expect(partialIdentity.balance).to.equal(BigInt(1000));
      expect(partialIdentity.revision).to.equal(BigInt(5));
      expect(Array.from(partialIdentity.notFoundPublicKeys)).to.deep.equal([10, 20]);
    });
  });

  describe('toJSON()', () => {
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

  describe('toObject()', () => {
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

  describe('id', () => {
    it('should get id', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      expect(partialIdentity.id.toBase58()).to.equal(identifier);
    });

    it('should set id', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      const newId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
      partialIdentity.id = newId;

      expect(partialIdentity.id.toBase58()).to.equal(newId);
    });
  });

  describe('loadedPublicKeys', () => {
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

    it('should set loadedPublicKeys', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      const pubKey = new wasm.IdentityPublicKey({
        keyId,
        purpose,
        securityLevel,
        keyType,
        isReadOnly: false,
        data: binaryData,
      });

      partialIdentity.loadedPublicKeys = { [keyId]: pubKey };

      const keys = partialIdentity.loadedPublicKeys;
      expect(Object.keys(keys)).to.deep.equal([String(keyId)]);
      expect(keys[String(keyId)].__type).to.equal('IdentityPublicKey');
    });
  });

  describe('balance', () => {
    it('should get balance', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
        balance: BigInt(500),
      });

      expect(partialIdentity.balance).to.equal(BigInt(500));
    });

    it('should set balance', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      partialIdentity.balance = BigInt(999);

      expect(partialIdentity.balance).to.equal(BigInt(999));
    });
  });

  describe('revision', () => {
    it('should get revision', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
        revision: BigInt(10),
      });

      expect(partialIdentity.revision).to.equal(BigInt(10));
    });

    it('should set revision', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
      });

      partialIdentity.revision = BigInt(42);

      expect(partialIdentity.revision).to.equal(BigInt(42));
    });
  });

  describe('notFoundPublicKeys', () => {
    it('should get notFoundPublicKeys', () => {
      const partialIdentity = new wasm.PartialIdentity({
        id: identifier,
        loadedPublicKeys: {},
        notFoundPublicKeys: [5, 15, 25],
      });

      expect(Array.from(partialIdentity.notFoundPublicKeys)).to.deep.equal([5, 15, 25]);
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

  describe('fromObject()', () => {
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

    it('should deserialize from hardcoded Object fixture and verify all getters', () => {
      const objectFixture = {
        id: Uint8Array.from(identifierBytes),
        loadedPublicKeys: {
          '2': {
            $formatVersion: '0',
            id: 2,
            purpose: 0,
            securityLevel: 1,
            type: 0,
            readOnly: false,
            data: Uint8Array.from([
              3, 106, 57, 67, 18, 228, 14, 129, 217, 40, 253, 226, 189, 231, 136,
              0, 112, 228, 250, 156, 29, 29, 155, 22, 141, 167, 7, 234, 70, 138,
              250, 43, 72,
            ]),
          },
        },
        balance: 1000n,
        revision: 5n,
        notFoundPublicKeys: [10, 20],
      };

      const restored = wasm.PartialIdentity.fromObject(objectFixture);

      expect(restored.id.toBase58()).to.equal(identifier);
      expect(Object.keys(restored.loadedPublicKeys)).to.deep.equal(['2']);
      const loadedKey = restored.loadedPublicKeys['2'];
      expect(loadedKey.__type).to.equal('IdentityPublicKey');
      expect(loadedKey.keyId).to.equal(2);
      expect(loadedKey.purpose).to.equal('AUTHENTICATION');
      expect(loadedKey.securityLevel).to.equal('CRITICAL');
      expect(loadedKey.keyType).to.equal('ECDSA_SECP256K1');
      expect(loadedKey.isReadOnly).to.equal(false);
      expect(restored.balance).to.equal(1000n);
      expect(restored.revision).to.equal(5n);
      expect(Array.from(restored.notFoundPublicKeys)).to.deep.equal([10, 20]);
    });
  });

  describe('fromJSON()', () => {
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

    it('should deserialize from hardcoded JSON fixture and verify all getters', () => {
      const jsonFixture = {
        id: 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1',
        loadedPublicKeys: {
          '2': {
            $formatVersion: '0',
            id: 2,
            purpose: 0,
            securityLevel: 1,
            type: 0,
            readOnly: false,
            data: 'A2o5QxLkDoHZKP3iveeIAHDk+pwdHZsWjacH6kaK+itI',
          },
        },
        balance: "1000",
        revision: "5",
        notFoundPublicKeys: [10, 20],
      };

      const restored = wasm.PartialIdentity.fromJSON(jsonFixture);

      expect(restored.id.toBase58()).to.equal(identifier);
      expect(Object.keys(restored.loadedPublicKeys)).to.deep.equal(['2']);
      const loadedKey = restored.loadedPublicKeys['2'];
      expect(loadedKey.__type).to.equal('IdentityPublicKey');
      expect(loadedKey.keyId).to.equal(2);
      expect(loadedKey.purpose).to.equal('AUTHENTICATION');
      expect(loadedKey.securityLevel).to.equal('CRITICAL');
      expect(loadedKey.keyType).to.equal('ECDSA_SECP256K1');
      expect(loadedKey.isReadOnly).to.equal(false);
      expect(restored.balance).to.equal(1000n);
      expect(restored.revision).to.equal(5n);
      expect(Array.from(restored.notFoundPublicKeys)).to.deep.equal([10, 20]);
    });
  });
});
