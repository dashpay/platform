const rewiremock = require('rewiremock/node');

const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

describe('Identity', () => {
  let rawIdentity;
  let identity;
  let Identity;
  let hashMock;
  let encodeMock;

  beforeEach(function beforeEach() {
    rawIdentity = {
      id: 'someId',
      type: 0,
      publicKeys: [
        {
          id: 1,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: 'somePublicKey',
          isEnabled: true,
        },
      ],
    };

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

    identity = new Identity(rawIdentity);
  });

  describe('#constructor', () => {
    it('should not set anything if nothing passed', () => {
      const instance = new Identity();

      expect(instance.id).to.be.undefined();
      expect(instance.type).to.be.undefined();
      expect(instance.publicKeys).to.deep.equal([]);
    });

    it('should set variables from raw model', () => {
      const instance = new Identity(rawIdentity);

      expect(instance.id).to.equal(rawIdentity.id);
      expect(instance.type).to.equal(rawIdentity.type);
      expect(instance.publicKeys).to.deep.equal(
        rawIdentity.publicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    });
  });

  describe('#getId', () => {
    it('should return set id', () => {
      expect(identity.getId()).to.equal(rawIdentity.id);
    });
  });

  describe('#getType', () => {
    it('should return set identity type', () => {
      identity.type = 42;
      expect(identity.getType()).to.equal(42);
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
      const key = identity.getPublicKeyById(1);

      expect(key).to.be.deep.equal(new IdentityPublicKey(rawIdentity.publicKeys[0]));
    });

    it("should return undefined if there's no key with such id", () => {
      const key = identity.getPublicKeyById(3);
      expect(key).to.be.undefined();
    });
  });

  describe('#serialize', () => {
    it('should return encoded json object', () => {
      encodeMock.returns(42); // for example
      const result = identity.serialize();

      expect(encodeMock).to.have.been.calledOnceWith(identity.toJSON());
      expect(result).to.equal(42);
    });
  });

  describe('#hash', () => {
    it('should return hex string of a buffer return by serialize', () => {
      const buffer = Buffer.from('someString');
      const bufferHex = buffer.toString('hex');

      encodeMock.returns(buffer);
      hashMock.returns(buffer);

      const result = identity.hash();

      expect(encodeMock).to.have.been.calledOnceWith(identity.toJSON());
      expect(hashMock).to.have.been.calledOnceWith(buffer);
      expect(result).to.equal(bufferHex);
    });
  });

  describe('#toJSON', () => {
    it('should return json representation', () => {
      const json = identity.toJSON();

      expect(json).to.deep.equal(rawIdentity);
    });
  });
});
