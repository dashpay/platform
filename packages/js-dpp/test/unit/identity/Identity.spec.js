const rewiremock = require('rewiremock/node');

const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

describe('Identity', () => {
  let rawIdentity;
  let identity;
  let Identity;
  let hashMock;
  let encodeMock;

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
      protocolVersion: Identity.PROTOCOL_VERSION,
      id: 'someId',
      publicKeys: [
        {
          id: 0,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: 'somePublicKey',
        },
      ],
      balance: 0,
    };

    identity = new Identity(rawIdentity);
  });

  describe('#constructor', () => {
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
});
