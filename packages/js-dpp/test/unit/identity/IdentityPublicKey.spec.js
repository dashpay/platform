const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');
const EmptyPublicKeyDataError = require('../../../lib/identity/errors/EmptyPublicKeyDataError');

describe('IdentityPublicKey', () => {
  let rawPublicKey;
  let publicKey;

  beforeEach(() => {
    rawPublicKey = {
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: 'somePublicKey',
    };

    publicKey = new IdentityPublicKey(rawPublicKey);
  });

  describe('#constructor', () => {
    it('should not set anything if nothing passed', () => {
      const instance = new IdentityPublicKey();

      expect(instance.id).to.be.undefined();
      expect(instance.type).to.be.undefined();
      expect(instance.data).to.be.undefined();
      expect(instance.enabled).to.be.true();
    });

    it('should set variables from raw model', () => {
      const instance = new IdentityPublicKey(rawPublicKey);

      expect(instance.id).to.equal(rawPublicKey.id);
      expect(instance.type).to.equal(rawPublicKey.type);
      expect(instance.data).to.deep.equal(rawPublicKey.data);
    });
  });

  describe('#getId', () => {
    it('should return set id', () => {
      expect(publicKey.getId()).to.equal(rawPublicKey.id);
    });
  });

  describe('#setId', () => {
    it('should set id', () => {
      publicKey.setId(42);

      expect(publicKey.id).to.equal(42);
    });
  });

  describe('#getType', () => {
    it('should return set type', () => {
      publicKey.type = 42;

      expect(publicKey.getType()).to.equal(42);
    });
  });

  describe('#setType', () => {
    it('should set type', () => {
      publicKey.setType(42);

      expect(publicKey.type).to.equal(42);
    });
  });

  describe('#getData', () => {
    it('should return set data', () => {
      expect(publicKey.getData()).to.equal(rawPublicKey.data);
    });
  });

  describe('#setData', () => {
    it('should set data', () => {
      publicKey.setData('42');

      expect(publicKey.data).to.equal('42');
    });
  });

  describe('#hash', () => {
    it('should return original public key hash', () => {
      publicKey = new IdentityPublicKey({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: 'Az7vgL9THE2e1nq8zwK98gx5Oy6Tro3pxc8PQxJA+oTx',
      });

      const result = publicKey.hash();

      expect(result).to.deep.equal('24940ae1982187675fc3ad95aac68769322d95f2');
    });

    it('should throw invalid argument error if data was not originally provided', () => {
      publicKey = new IdentityPublicKey({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      });

      try {
        publicKey.hash();
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(EmptyPublicKeyDataError);
        expect(e.message).to.equal(
          'Public key data is not set',
        );
      }
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation', () => {
      const json = publicKey.toJSON();

      expect(json).to.deep.equal(rawPublicKey);
    });
  });
});
