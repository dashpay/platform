const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');
const EmptyPublicKeyDataError = require('../../../lib/identity/errors/EmptyPublicKeyDataError');

describe('IdentityPublicKey', () => {
  let rawPublicKey;
  let publicKey;

  beforeEach(() => {
    rawPublicKey = {
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64'),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
    };

    publicKey = new IdentityPublicKey(rawPublicKey);
  });

  describe('#constructor', () => {
    it('should not set anything if nothing passed', () => {
      const instance = new IdentityPublicKey();

      expect(instance.id).to.be.undefined();
      expect(instance.type).to.be.undefined();
      expect(instance.data).to.be.undefined();
    });

    it('should set variables from raw model', () => {
      const instance = new IdentityPublicKey(rawPublicKey);

      expect(instance.id).to.equal(rawPublicKey.id);
      expect(instance.type).to.equal(rawPublicKey.type);
      expect(instance.data).to.equal(rawPublicKey.data);
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
      const buffer = Buffer.alloc(36);

      publicKey.setData(buffer);

      expect(publicKey.data).to.equal(buffer);
    });
  });

  describe('#getPurpose', () => {
    it('should return set data', () => {
      expect(publicKey.getPurpose()).to.equal(rawPublicKey.purpose);
    });
  });

  describe('#setPurpose', () => {
    it('should set data', () => {
      publicKey.setPurpose(IdentityPublicKey.PURPOSES.DECRYPTION);

      expect(publicKey.purpose).to.equal(IdentityPublicKey.PURPOSES.DECRYPTION);
    });
  });

  describe('#getSecurityLevel', () => {
    it('should return set data', () => {
      expect(publicKey.getSecurityLevel()).to.equal(rawPublicKey.securityLevel);
    });
  });

  describe('#setSecurityLevel', () => {
    it('should set data', () => {
      publicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);

      expect(publicKey.securityLevel).to.equal(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);
    });
  });

  describe('#hash', () => {
    it('should return original public key hash', () => {
      const result = publicKey.hash();

      const expectedHash = Buffer.from('Q/5mfilFPdZt+Fr5JWC1+tg0cPs=', 'base64');

      expect(result).to.deep.equal(expectedHash);
    });

    it('should return data in case ECDSA_HASH160', () => {
      rawPublicKey = {
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_HASH160,
        data: Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      };

      publicKey = new IdentityPublicKey(rawPublicKey);

      const result = publicKey.hash();

      const expectedHash = Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64');

      expect(result).to.deep.equal(expectedHash);
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
      const jsonPublicKey = publicKey.toJSON();

      expect(jsonPublicKey).to.deep.equal({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: 'AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH',
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      });
    });
  });
});
