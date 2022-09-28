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
      readOnly: false,
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

  describe('#isReadOnly', () => {
    it('should return readOnly', () => {
      expect(publicKey.isReadOnly()).to.equal(rawPublicKey.readOnly);
    });
  });

  describe('#setReadOnly', () => {
    it('should set readOnly', () => {
      publicKey.setReadOnly(true);

      expect(publicKey.readOnly).to.equal(true);
    });
  });

  describe('#setDisabledAt', () => {
    it('should set disabledAt', () => {
      publicKey.setDisabledAt(123);

      expect(publicKey.disabledAt).to.equal(123);
    });
  });

  describe('#getDisabledAt', () => {
    it('should return disabledAt', () => {
      publicKey.disabledAt = 42;

      expect(publicKey.getDisabledAt()).to.equal(42);
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

    it('should return original public key hash in case BLS12_381', () => {
      rawPublicKey = {
        id: 0,
        type: IdentityPublicKey.TYPES.BLS12_381,
        data: Buffer.from('01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694', 'hex'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      };

      publicKey = new IdentityPublicKey(rawPublicKey);

      const result = publicKey.hash();

      const expectedHash = Buffer.from('1de31a0a328e8822f9cb2c25141d7d80baee26ef', 'hex');

      expect(result).to.deep.equal(expectedHash);
    });

    it('should return data in case BIP13_SCRIPT_HASH', () => {
      rawPublicKey = {
        id: 0,
        type: IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH,
        data: Buffer.from('54c557e07dde5bb6cb791c7a540e0a4796f5e97e', 'hex'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      };

      publicKey = new IdentityPublicKey(rawPublicKey);

      const result = publicKey.hash();

      const expectedHash = Buffer.from('54c557e07dde5bb6cb791c7a540e0a4796f5e97e', 'hex');

      expect(result).to.deep.equal(expectedHash);
    });

    it('should throw invalid argument error if data was not originally provided', async () => {
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
        readOnly: false,
      });
    });

    it('should return JSON representation with optional properties', () => {
      publicKey.setDisabledAt(42);

      const jsonPublicKey = publicKey.toJSON();

      expect(jsonPublicKey).to.deep.equal({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: 'AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH',
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
        disabledAt: 42,
      });
    });
  });

  describe('#isMaster', () => {
    it('should return true when public key has MASTER security level', () => {
      publicKey.securityLevel = IdentityPublicKey.SECURITY_LEVELS.MASTER;

      const result = publicKey.isMaster();

      expect(result).to.be.true();
    });

    it('should return false when public key doesn\'t have MASTER security level', () => {
      publicKey.securityLevel = IdentityPublicKey.SECURITY_LEVELS.HIGH;

      const result = publicKey.isMaster();

      expect(result).to.be.false();
    });
  });
});
