const { default: loadWasmDpp } = require('../../../dist');
const generateRandomIdentifierAsync = require('../../../lib/test/utils/generateRandomIdentifierAsync');

const lodashCloneDeep = require('lodash.clonedeep');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const OldIdentity = require('@dashevo/dpp/lib/identity/Identity');

const serializer = require('@dashevo/dpp/lib/util/serializer');
const { hash: hashFunction } = require('@dashevo/dpp/lib/util/hash');
const hash = require('@dashevo/dpp/lib/util/hash');
const { expect } = require('chai');

describe('Identity', () => {
  let rawIdentity;
  let identity;
  let hashMock;
  let encodeMock;
  let metadataFixture;
  let Identity;
  let Metadata;
  let IdentityPublicKeyWasm;

  before(async () => {
    ({ Identifier, Identity, Metadata, IdentityPublicKey: IdentityPublicKeyWasm } = await loadWasmDpp());
  });


  beforeEach(async function beforeEach() {
    rawIdentity = {
      protocolVersion: protocolVersion.latestVersion,
      id: await generateRandomIdentifierAsync(),
      publicKeys: [
        {
          id: 0,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: Buffer.alloc(36).fill('a'),
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
          readOnly: false,
          signature: Buffer.alloc(36).fill('a'),
        },
      ],
      balance: 0,
      revision: 0,
    };

    identity = new Identity(rawIdentity);

    metadataFixture = new Metadata(42, 0);

    identity.setMetadata(metadataFixture);

    metadataFixture = new Metadata(42, 0);

    // encodeMock = this.sinonSandbox.stub(serializer, 'encode');
    // hashMock = this.sinonSandbox.stub(hash, 'hash');
  });

  afterEach(() => {
    // encodeMock.restore();
    // hashMock.restore();
  });

  describe('#constructor', () => {
    it('should set variables from raw model', () => {
      const instance = new Identity(rawIdentity);

      expect(instance.getId().toBuffer()).to.deep.equal(rawIdentity.id.toBuffer());
      expect(instance.getPublicKeys().map((pk) => pk.toObject())).to.deep.equal(
        rawIdentity.publicKeys.map((rawPublicKey) => new IdentityPublicKeyWasm(rawPublicKey).toObject()),
      );
    });
  });

  describe('#getId', () => {
    it('should return set id', () => {
      identity = new Identity(rawIdentity);
      expect(identity.getId().toBuffer()).to.deep.equal(rawIdentity.id.toBuffer());
    });
  });

  describe('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(identity.getPublicKeys().map(pk => pk.toObject())).to.deep.equal(
        rawIdentity.publicKeys.map((rawPublicKey) => new IdentityPublicKeyWasm(rawPublicKey).toObject()),
      );
    });
  });

  describe('#setPublicKeys', () => {
    it('should reject input which is not array of public keys', () => {
      expect(() => { identity.setPublicKeys(42) })
        .throws("Setting public keys failed. The input ('42') is invalid. You must use array of PublicKeys");
      expect(identity.getPublicKeys()).length(1)
    });

    it('should set public keys', () => {
      const ipk = new IdentityPublicKey({
        id: 2,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.alloc(36).fill('a'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        signature: Buffer.alloc(36).fill('a'),
        readOnly: false,
      });

      identity.setPublicKeys([ipk]);
      expect(identity.getPublicKeys()).length(1);
      expect(identity.getPublicKeys()[0].getId()).eq(2);
    });
  });

  describe('#getPublicKeyById', () => {
    it('should return a public key for a given id', () => {
      const key = identity.getPublicKeyById(0);

      expect(key.toObject()).to.be.deep.equal(new IdentityPublicKeyWasm(rawIdentity.publicKeys[0]).toObject());
    });

    it("should return undefined if there's no key with such id", () => {
      const key = identity.getPublicKeyById(3);
      expect(key).to.be.undefined();
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized identity', () => {
      const oldIdentity = new OldIdentity(rawIdentity);
      const result = Buffer.from(identity.toBuffer());

      expect(result).to.deep.eq(oldIdentity.toBuffer());
    });
  });

  describe('#hash', () => {
    it('should return hex string of a buffer return by serialize', () => {
      const expected_hash = hashFunction(identity.toBuffer());
      const result = identity.hash();

      const identityDataToEncode = identity.toObject();
      delete identityDataToEncode.protocolVersion;

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32LE(identity.getProtocolVersion(), 0);

      expect(result).to.deep.equal(expected_hash);
    });
  });

  describe('#toObject', () => {
    it('should return plain object representation', () => {
      let identityObject = identity.toObject();

      //! TODO The structures exported from WASM cannot be deeply inspected and hence: compared.
      //! TODO The WASM structure contains `ptr` field with a pointer to memory in WASM space, and
      //! TODO the address is always different
      identityObject.id = identityObject.id.toJSON();
      rawIdentity.id = rawIdentity.id.toJSON();

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
            type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
            data: rawIdentity.publicKeys[0].data.toString('base64'),
            purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
            securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
            signature: rawIdentity.publicKeys[0].signature.toString('base64'),
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
      const otherMetadata = new Metadata(43, 1);
      const expectedMetadata = new Metadata(43, 1);

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

      identity.addPublicKeys(
        new IdentityPublicKeyWasm({
          id: 99,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: Buffer.alloc(36).fill('a'),
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
          signature: Buffer.alloc(36).fill('a'),
          readOnly: false,
        }),
        new IdentityPublicKeyWasm({
          id: 50,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: Buffer.alloc(36).fill('a'),
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
          signature: Buffer.alloc(36).fill('a'),
          readOnly: false,
        })
      );

      const maxId = identity.getPublicKeyMaxId();

      const publicKeyIds = identity.getPublicKeys().map((publicKey) => publicKey.getId());

      expect(Math.max(...publicKeyIds)).to.equal(99);
      expect(Math.max(...publicKeyIds)).to.equal(maxId);
    });
  });
});
