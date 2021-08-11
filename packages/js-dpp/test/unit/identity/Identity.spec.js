const rewiremock = require('rewiremock/node');

const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');

const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');
const Metadata = require('../../../lib/Metadata');
const protocolVersion = require('../../../lib/protocolVersion');

describe('Identity', () => {
  let rawIdentity;
  let identity;
  let Identity;
  let hashMock;
  let encodeMock;
  let metadataFixture;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    encodeMock = this.sinonSandbox.stub();

    Identity = rewiremock.proxy(
      '../../../lib/identity/Identity',
      {
        '../../../lib/util/hash': hashMock,
        '../../../lib/util/serializer': {
          encode: encodeMock,
        },
      },
    );

    rawIdentity = {
      protocolVersion: protocolVersion.latestVersion,
      id: generateRandomIdentifier(),
      publicKeys: [
        {
          id: 0,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: Buffer.alloc(36).fill('a'),
        },
      ],
      balance: 0,
      revision: 0,
    };

    identity = new Identity(rawIdentity);

    metadataFixture = new Metadata(42, 0);

    identity.setMetadata(metadataFixture);
  });

  describe('#constructor', () => {
    it('should set variables from raw model', () => {
      const instance = new Identity(rawIdentity);

      expect(instance.id).to.deep.equal(rawIdentity.id);
      expect(instance.type).to.equal(rawIdentity.type);
      expect(instance.publicKeys).to.deep.equal(
        rawIdentity.publicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    });
  });

  describe('#getId', () => {
    it('should return set id', () => {
      expect(identity.getId()).to.deep.equal(rawIdentity.id);
    });
  });

  describe('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(identity.getPublicKeys()).to.deep.equal(
        rawIdentity.publicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    });
  });

  describe('#setPublicKeys', () => {
    it('should set public keys', () => {
      identity.setPublicKeys(42);
      expect(identity.publicKeys).to.equal(42);
    });
  });

  describe('#getPublicKeyById', () => {
    it('should return a public key for a given id', () => {
      const key = identity.getPublicKeyById(0);

      expect(key).to.be.deep.equal(new IdentityPublicKey(rawIdentity.publicKeys[0]));
    });

    it("should return undefined if there's no key with such id", () => {
      const key = identity.getPublicKeyById(3);
      expect(key).to.be.undefined();
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized Identity', () => {
      const encodeMockData = Buffer.from('42');
      encodeMock.returns(encodeMockData); // for example

      const result = identity.toBuffer();

      const identityDataToEncode = identity.toObject();
      delete identityDataToEncode.protocolVersion;

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32BE(identity.getProtocolVersion(), 0);

      expect(encodeMock).to.have.been.calledOnceWith(identityDataToEncode);
      expect(result).to.deep.equal(Buffer.concat([protocolVersionUInt32, encodeMockData]));
    });
  });

  describe('#hash', () => {
    it('should return hex string of a buffer return by serialize', () => {
      const buffer = Buffer.from('someString');

      encodeMock.returns(buffer);
      hashMock.returns(buffer);

      const result = identity.hash();

      const identityDataToEncode = identity.toObject();
      delete identityDataToEncode.protocolVersion;

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32BE(identity.getProtocolVersion(), 0);

      expect(encodeMock).to.have.been.calledOnceWith(identityDataToEncode);
      expect(hashMock).to.have.been.calledOnceWith(Buffer.concat([protocolVersionUInt32, buffer]));
      expect(result).to.equal(buffer);
    });
  });

  describe('#toObject', () => {
    it('should return plain object representation', () => {
      expect(identity.toObject()).to.deep.equal(rawIdentity);
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
          },
        ],
        balance: 0,
        revision: 0,
      });
    });
  });

  describe('#getBalance', () => {
    it('should return set identity balance', () => {
      identity.balance = 42;
      expect(identity.getBalance()).to.equal(42);
    });
  });

  describe('#setBalance', () => {
    it('should set identity balance', () => {
      identity.setBalance(42);
      expect(identity.balance).to.equal(42);
    });
  });

  describe('#increaseBalance', () => {
    it('should increase identity balance', () => {
      const result = identity.increaseBalance(42);

      expect(result).to.equal(42);
      expect(identity.balance).to.equal(42);
    });
  });

  describe('#reduceBalance', () => {
    it('should reduce identity balance', () => {
      identity.balance = 42;

      const result = identity.reduceBalance(2);

      expect(result).to.equal(40);
      expect(identity.balance).to.equal(40);
    });
  });

  describe('#setMetadata', () => {
    it('should set metadata', () => {
      const otherMetadata = new Metadata(43, 1);

      identity.setMetadata(otherMetadata);

      expect(identity.metadata).to.deep.equal(otherMetadata);
    });
  });

  describe('#getMetadata', () => {
    it('should get metadata', () => {
      expect(identity.getMetadata()).to.deep.equal(metadataFixture);
    });
  });
});
