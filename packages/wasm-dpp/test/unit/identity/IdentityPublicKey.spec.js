const { IdentityPublicKey } = require('@dashevo/wasm-dpp');

describe('IdentityPublicKey', () => {
  let rawPublicKey;
  let publicKey;

  beforeEach(() => {
    const keyData = Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64');

    rawPublicKey = {
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: keyData,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
      readOnly: false,
      contractBounds: null,
    };

    publicKey = new IdentityPublicKey(1);
    publicKey.setData(keyData);
  });

  describe('#constructor', () => {
    it('should set variables from raw model', () => {
      expect(publicKey.getId()).to.equal(rawPublicKey.id);
      expect(publicKey.getType()).to.equal(rawPublicKey.type);
      expect(publicKey.getData()).to.deep.equal(rawPublicKey.data);
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

      expect(publicKey.getId()).to.equal(42);
    });
  });

  describe('#getType', () => {
    it('should return set type', () => {
      publicKey.setType(3);

      expect(publicKey.getType()).to.equal(3);
    });
  });

  describe('#setType', () => {
    it('should set type', () => {
      publicKey.setType(3);

      expect(publicKey.getType()).to.equal(3);
    });
  });

  describe('#getData', () => {
    it('should return set data', () => {
      expect(publicKey.getData()).to.be.deep.equal(rawPublicKey.data);
    });
  });

  describe('#setData', () => {
    it('should set data', () => {
      const buffer = Buffer.alloc(36);

      publicKey.setData(buffer);

      expect(publicKey.getData()).to.be.deep.equal(buffer);
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

      expect(publicKey.getPurpose()).to.equal(IdentityPublicKey.PURPOSES.DECRYPTION);
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

      expect(publicKey.getSecurityLevel()).to.equal(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);
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

      expect(publicKey.isReadOnly()).to.equal(true);
    });
  });

  describe('#setDisabledAt', () => {
    it('should set disabledAt', () => {
      const now = new Date();
      publicKey.setDisabledAt(now);

      expect(publicKey.getDisabledAt()).to.deep.equal(now);
    });
  });

  describe('#getDisabledAt', () => {
    it('should return disabledAt', () => {
      const now = new Date();
      publicKey.setDisabledAt(now);

      expect(publicKey.getDisabledAt()).to.deep.equal(now);
    });
  });

  describe('#hash', () => {
    it('should return original public key hash', () => {
      const result = publicKey.hash();

      const expectedHash = Buffer.from('Q/5mfilFPdZt+Fr5JWC1+tg0cPs=', 'base64');

      expect(result).to.deep.equal(expectedHash);
    });

    it('should return data in case ECDSA_HASH160', () => {
      publicKey = new IdentityPublicKey(1);
      publicKey.setType(IdentityPublicKey.TYPES.ECDSA_HASH160);
      publicKey.setData(Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTdi', 'base64'));

      const result = publicKey.hash();

      const expectedHash = Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTdg=', 'base64');

      expect(result).to.deep.equal(expectedHash);
    });

    it('should return original public key hash in case BLS12_381', () => {
      publicKey = new IdentityPublicKey(1);
      publicKey.setType(IdentityPublicKey.TYPES.BLS12_381);
      publicKey.setData(Buffer.from('01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694', 'hex'));

      const result = publicKey.hash();

      const expectedHash = Buffer.from('1de31a0a328e8822f9cb2c25141d7d80baee26ef', 'hex');

      expect(result).to.deep.equal(expectedHash);
    });

    it('should return data in case BIP13_SCRIPT_HASH', () => {
      publicKey = new IdentityPublicKey(1);
      publicKey.setType(IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH);
      publicKey.setData(Buffer.from('54c557e07dde5bb6cb791c7a540e0a4796f5e97e', 'hex'));

      const result = publicKey.hash();

      const expectedHash = Buffer.from('54c557e07dde5bb6cb791c7a540e0a4796f5e97e', 'hex');

      expect(result).to.deep.equal(expectedHash);
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation', () => {
      const now = new Date();
      publicKey.setDisabledAt(now);

      const jsonPublicKey = publicKey.toJSON();

      expect(jsonPublicKey).to.deep.equal({
        $version: '0',
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: 'AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH',
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
        readOnly: false,
        disabledAt: now.getTime(),
        contractBounds: null,
      });
    });
  });

  describe('#toObject', () => {
    it('should return object representation', () => {
      const now = new Date();
      publicKey.setDisabledAt(now);

      const jsonPublicKey = publicKey.toObject();

      expect(jsonPublicKey).to.deep.equal({
        $version: '0',
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
        readOnly: false,
        disabledAt: now.getTime(),
        contractBounds: null,
      });
    });
  });

  describe('#isMaster', () => {
    it('should return true when public key has MASTER security level', () => {
      publicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

      const result = publicKey.isMaster();

      expect(result).to.be.true();
    });

    it('should return false when public key doesn\'t have MASTER security level', () => {
      publicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.HIGH);

      const result = publicKey.isMaster();

      expect(result).to.be.false();
    });
  });
});
