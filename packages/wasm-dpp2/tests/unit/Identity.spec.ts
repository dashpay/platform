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
  keyId, purpose, securityLevel, keyType, binaryData, binaryDataHex,
} from './mocks/PublicKey/index.js';

before(async () => {
  await initWasm();
});

describe('Identity', () => {
  // Helper to create a fully-populated identity for fixture tests
  function createPopulatedIdentity() {
    const identity = new wasm.Identity(identifier);
    identity.balance = balance; // BigInt(100)
    identity.revision = revision; // BigInt(99111)

    const pubKey = new wasm.IdentityPublicKey({
      keyId, // 2
      purpose, // 'AUTHENTICATION' -> 0
      securityLevel, // 'CRITICAL' -> 1
      keyType, // 'ECDSA_SECP256K1' -> 0
      isReadOnly: false,
      data: binaryData, // 33-byte compressed secp256k1 pubkey
    });
    identity.addPublicKey(pubKey);

    return identity;
  }

  // Expected JSON representation (toJSON output - u64 fields as strings).
  // After Phase D step 4, `disabledAt: null` is stripped at the rs-dpp layer
  // for non-disabled keys via `#[serde(skip_serializing_if = "Option::is_none")]`.
  const expectedJSONOutput = {
    $formatVersion: '0',
    id: identifier,
    publicKeys: [
      {
        $formatVersion: '0',
        id: keyId,
        purpose: 0,
        securityLevel: 1,
        contractBounds: null,
        type: 0,
        readOnly: false,
        data: 'A2o5QxLkDoHZKP3iveeIAHDk+pwdHZsWjacH6kaK+itI',
      },
    ],
    balance: 100,
    revision: 99111,
  };

  // Expected JSON for fromJSON input - u64 fields as numbers (tagged enum serde limitation)
  const expectedJSONInput = {
    $formatVersion: '0',
    id: identifier,
    publicKeys: [
      {
        $formatVersion: '0',
        id: keyId,
        purpose: 0,
        securityLevel: 1,
        contractBounds: null,
        type: 0,
        readOnly: false,
        data: 'A2o5QxLkDoHZKP3iveeIAHDk+pwdHZsWjacH6kaK+itI',
      },
    ],
    balance: 100,
    revision: 99111,
  };

  // Expected Object representation. `disabledAt` is also stripped on the
  // value path now (same `skip_serializing_if` attribute applies to both
  // serde_json and platform_value paths).
  const expectedObject = {
    $formatVersion: '0',
    id: Uint8Array.from(identifierBytes),
    publicKeys: [
      {
        $formatVersion: '0',
        id: keyId,
        purpose: 0,
        securityLevel: 1,
        contractBounds: undefined,
        type: 0,
        readOnly: false,
        data: Buffer.from(binaryDataHex, 'hex'),
      },
    ],
    balance: BigInt(100),
    revision: BigInt(99111),
  };

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

    it('should match expected JSON for populated identity', () => {
      const identity = createPopulatedIdentity();
      const json = identity.toJSON();

      expect(json).to.deep.equal(expectedJSONOutput);
    });
  });

  describe('fromJSON()', () => {
    it('should recreate identity from JSON', () => {
      const identity = new wasm.Identity(identifier);
      const identityJson = identity.toJSON();
      const restoredIdentity = wasm.Identity.fromJSON(identityJson);

      expect(Array.from(restoredIdentity.toBytes())).to.deep.equal(Array.from(identity.toBytes()));
    });

    it('should restore identity from JSON fixture and verify via getters', () => {
      const identity = wasm.Identity.fromJSON(expectedJSONInput);

      expect(identity.id.toBase58()).to.equal(identifier);
      expect(identity.balance).to.equal(balance);
      expect(identity.revision).to.equal(revision);
      expect(identity.publicKeys.length).to.equal(1);

      const key = identity.getPublicKeyById(keyId);
      expect(key.keyId).to.equal(keyId);
      expect(key.purpose).to.equal('AUTHENTICATION');
      expect(key.securityLevel).to.equal('CRITICAL');
      expect(key.keyType).to.equal('ECDSA_SECP256K1');
      expect(key.isReadOnly).to.equal(false);
      expect(key.data).to.equal(binaryDataHex);
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

    it('should match expected Object for populated identity', () => {
      const identity = createPopulatedIdentity();
      const obj = identity.toObject();

      expect(obj).to.deep.equal(expectedObject);
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

    it('should restore identity from toObject output and verify via getters', () => {
      const original = createPopulatedIdentity();
      const obj = original.toObject();
      // Identity.fromObject uses serde which expects data as base64 string, not Uint8Array
      for (const pk of obj.publicKeys) {
        if (pk.data instanceof Uint8Array) {
          pk.data = Buffer.from(pk.data).toString('base64');
        }
      }
      const identity = wasm.Identity.fromObject(obj);

      expect(identity.id.toBase58()).to.equal(identifier);
      expect(identity.balance).to.equal(balance);
      expect(identity.revision).to.equal(revision);
      expect(identity.publicKeys.length).to.equal(1);

      const key = identity.getPublicKeyById(keyId);
      expect(key.keyId).to.equal(keyId);
      expect(key.purpose).to.equal('AUTHENTICATION');
      expect(key.securityLevel).to.equal('CRITICAL');
      expect(key.keyType).to.equal('ECDSA_SECP256K1');
      expect(key.isReadOnly).to.equal(false);
      expect(key.data).to.equal(binaryDataHex);
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
