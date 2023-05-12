const varint = require('varint');
const JSIdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { hash: hashFunction } = require('@dashevo/dpp/lib/util/hash');
const { expect } = require('chai');
const generateRandomIdentifierAsync = require('../../../lib/test/utils/generateRandomIdentifierAsync');
const { default: loadWasmDpp } = require('../../../dist');

describe('Identity', () => {
  let rawIdentity;
  let identity;
  let metadataFixture;
  let Identity;
  let Metadata;
  let IdentityPublicKey;
  let KeyPurpose;
  let KeyType;
  let KeySecurityLevel;

  before(async () => {
    ({
      Identity, Metadata, IdentityPublicKey, KeyPurpose, KeyType, KeySecurityLevel,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    rawIdentity = {
      protocolVersion: protocolVersion.latestVersion,
      id: await generateRandomIdentifierAsync(),
      publicKeys: [
        {
          id: 0,
          type: KeyType.ECDSA_SECP256K1,
          data: Buffer.alloc(36).fill('a'),
          purpose: KeyPurpose.AUTHENTICATION,
          securityLevel: KeySecurityLevel.MASTER,
          readOnly: false,
        },
      ],
      balance: 0,
      revision: 0,
    };

    identity = new Identity(rawIdentity);

    metadataFixture = new Metadata({
      blockHeight: 42,
      coreChainLockedHeight: 0,
      timeMs: 100,
      protocolVersion: 2,
    });

    identity.setMetadata(metadataFixture);

    metadataFixture = new Metadata({
      blockHeight: 42,
      coreChainLockedHeight: 0,
      timeMs: 100,
      protocolVersion: 2,
    });
  });

  afterEach(() => {
  });

  describe('#constructor', () => {
    it('should set variables from raw model', () => {
      const instance = identity;

      expect(instance.getId().toBuffer()).to.deep.equal(rawIdentity.id.toBuffer());
      expect(
        instance.getPublicKeys().map((pk) => pk.toObject()),
      ).to.deep.equal(
        rawIdentity.publicKeys,
      );
    });
  });

  describe('#getId', () => {
    it('should return set id', () => {
      expect(identity.getId().toBuffer()).to.deep.equal(rawIdentity.id.toBuffer());
    });
  });

  describe('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(identity.getPublicKeys().map((pk) => pk.toObject())).to.deep.equal(
        rawIdentity.publicKeys,
      );
    });
  });

  describe('#setPublicKeys', () => {
    it('should reject input which is not array of public keys', () => {
      expect(() => { identity.setPublicKeys(42); })
        .throws("Setting public keys failed. The input ('42') is invalid. You must use array of PublicKeys");
      expect(identity.getPublicKeys()).length(1);
    });

    it('should set public keys', () => {
      const rawKey = {
        id: 2,
        type: KeyType.ECDSA_SECP256K1,
        data: Buffer.alloc(36).fill('a'),
        purpose: KeyPurpose.AUTHENTICATION,
        securityLevel: KeySecurityLevel.MASTER,
        readOnly: false,
      };

      const ipk = new JSIdentityPublicKey(rawKey);

      identity.setPublicKeys([ipk]);
      expect(identity.getPublicKeys()).length(1);
      expect(identity.getPublicKeys()[0].toObject()).to.be.deep.equal(rawKey);
    });
  });

  describe('#getPublicKeyById', () => {
    it('should return a public key for a given id', () => {
      const key = identity.getPublicKeyById(0);

      expect(
        key.toObject(),
      ).to.be.deep.equal(
        rawIdentity.publicKeys[0],
      );
    });

    it("should return undefined if there's no key with such id", () => {
      const key = identity.getPublicKeyById(3);
      expect(key).to.be.undefined();
    });
  });

  describe('#toBuffer', () => {
    it('should return buffer', () => {
      const result = identity.toBuffer();
      expect(result).to.be.instanceOf(Buffer);
      expect(result).to.have.length(87);
    });
  });

  describe('#fromBuffer', () => {
    it('should re-create identity from buffer', () => {
      const buffer = identity.toBuffer();
      const recoveredIdentity = Identity.fromBuffer(buffer);
      expect(recoveredIdentity.toObject())
        .to.be.deep.equal(identity.toObject());
    });
  });

  describe('#hash', () => {
    it('should return the same has as JS Identity', () => {
      const expectedHash = hashFunction(identity.toBuffer());
      const result = identity.hash();

      const identityDataToEncode = identity.toObject();
      delete identityDataToEncode.protocolVersion;

      varint.encode(identity.getProtocolVersion());

      expect(result).to.deep.equal(expectedHash);
    });
  });

  describe('#toObject', () => {
    it('should return plain object representation', () => {
      const identityObject = identity.toObject();

      // We can't compare Identifier directly - it needs to be converted into a Buffer
      rawIdentity.id = rawIdentity.id.toBuffer();

      expect(identityObject).to.deep.equal(rawIdentity);
    });
  });

  describe('#toJSON', () => {
    it('should return json representation', () => {
      const jsonIdentity = identity.toJSON();

      expect(jsonIdentity).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        id: rawIdentity.id.toString(),
        publicKeys: [
          {
            id: 0,
            type: KeyType.ECDSA_SECP256K1,
            data: rawIdentity.publicKeys[0].data.toString('base64'),
            purpose: KeyPurpose.AUTHENTICATION,
            securityLevel: KeySecurityLevel.MASTER,
            readOnly: false,
          },
        ],
        balance: 0,
        revision: 0,
      });
    });
  });

  describe('#getBalance', () => {
    it('should return set identity balance', () => {
      identity.setBalance(42);
      expect(identity.getBalance()).to.equal(42);
    });
  });

  describe('#setBalance', () => {
    it('should set identity balance', () => {
      identity.setBalance(42);
      expect(identity.getBalance()).to.equal(42);
    });
  });

  describe('#increaseBalance', () => {
    it('should increase identity balance', () => {
      const result = identity.increaseBalance(42);

      expect(result).to.equal(42);
      expect(identity.getBalance()).to.equal(42);
    });
  });

  describe('#reduceBalance', () => {
    it('should reduce identity balance', () => {
      identity.setBalance(42);

      const result = identity.reduceBalance(2);

      expect(result).to.equal(40);
      expect(identity.getBalance()).to.equal(40);
    });
  });

  describe('#setMetadata', () => {
    it('should set metadata', () => {
      const otherMetadata = new Metadata({
        blockHeight: 43,
        coreChainLockedHeight: 1,
        timeMs: 100,
        protocolVersion: 2,
      });
      const expectedMetadata = new Metadata({
        blockHeight: 43,
        coreChainLockedHeight: 1,
        timeMs: 100,
        protocolVersion: 2,
      });

      identity.setMetadata(otherMetadata);

      expect(identity.getMetadata().toObject()).to.deep.equal(expectedMetadata.toObject());
    });
  });

  describe('#getMetadata', () => {
    it('should get metadata', () => {
      expect(identity.getMetadata().toObject()).to.deep.equal(metadataFixture.toObject());
    });
  });

  describe('#getPublicKeyMaxId', () => {
    it('should get the biggest public key ID', () => {
      const key = new IdentityPublicKey({
        id: 99,
        type: KeyType.ECDSA_SECP256K1,
        data: Buffer.alloc(36).fill('a'),
        purpose: KeyPurpose.AUTHENTICATION,
        securityLevel: KeySecurityLevel.MASTER,
        signature: Buffer.alloc(36).fill('a'),
        readOnly: false,
      });
      identity.addPublicKeys(
        key,
        new IdentityPublicKey({
          id: 50,
          type: KeyType.ECDSA_SECP256K1,
          data: Buffer.alloc(36).fill('a'),
          purpose: KeyPurpose.AUTHENTICATION,
          securityLevel: KeySecurityLevel.MASTER,
          signature: Buffer.alloc(36).fill('a'),
          readOnly: false,
        }),
      );

      const maxId = identity.getPublicKeyMaxId();

      const publicKeyIds = identity.getPublicKeys().map((publicKey) => publicKey.getId());

      expect(Math.max(...publicKeyIds)).to.equal(99);
      expect(Math.max(...publicKeyIds)).to.equal(maxId);
    });
  });
});
