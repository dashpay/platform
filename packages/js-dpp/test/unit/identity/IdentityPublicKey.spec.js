const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

describe('IdentityPublicKey', () => {
  let rawPublicKey;
  let publicKey;

  beforeEach(() => {
    rawPublicKey = {
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: 'somePublicKey',
      isEnabled: true,
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
      expect(instance.enabled).to.deep.equal(rawPublicKey.isEnabled);
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

  describe('#isEnabled', () => {
    it('should return set enabled', () => {
      expect(publicKey.isEnabled()).to.equal(rawPublicKey.isEnabled);
    });
  });

  describe('#setEnabled', () => {
    it('should set enabled', () => {
      publicKey.setEnabled(false);

      expect(publicKey.enabled).to.equal(false);
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation', () => {
      const json = publicKey.toJSON();

      expect(json).to.deep.equal(rawPublicKey);
    });
  });
});
