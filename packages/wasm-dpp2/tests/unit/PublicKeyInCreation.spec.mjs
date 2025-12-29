import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('IdentityPublicKeyInCreation', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect(publicKeyInCreation.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create from values and convert to identity public key', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      const publicKey = publicKeyInCreation.toIdentityPublicKey();

      expect(publicKeyInCreation.__wbg_ptr).to.not.equal(0);
      expect(publicKey.constructor.name).to.equal('IdentityPublicKey');
    });
  });

  describe('getters', () => {
    it('should allow to get key id', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect(publicKeyInCreation.keyId).to.equal(0);
    });

    it('should allow to get purpose', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect(publicKeyInCreation.purpose).to.equal('AUTHENTICATION');
    });

    it('should allow to get security level', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect(publicKeyInCreation.securityLevel).to.equal('MASTER');
    });

    it('should allow to get key type', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect(publicKeyInCreation.keyType).to.equal('ECDSA_SECP256K1');
    });

    it('should allow to get read only', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect(publicKeyInCreation.readOnly).to.equal(false);
    });

    it('should allow to get data', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect(Buffer.from(publicKeyInCreation.data)).to.deep.equal(Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'));
    });

    it('should allow to get signature', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      expect([...publicKeyInCreation.signature]).to.deep.equal([]);
    });
  });

  describe('setters', () => {
    it('should allow to set key id', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      publicKeyInCreation.keyId = 123;

      expect(publicKeyInCreation.keyId).to.equal(123);
    });

    it('should allow to set purpose', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      publicKeyInCreation.purpose = 'OWNER';

      expect(publicKeyInCreation.purpose).to.equal('OWNER');
    });

    it('should allow to set security level', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      publicKeyInCreation.securityLevel = 'critical';

      expect(publicKeyInCreation.securityLevel).to.equal('CRITICAL');
    });

    it('should allow to set key type', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      publicKeyInCreation.keyType = 'ECDSA_HASH160';

      expect(publicKeyInCreation.keyType).to.equal('ECDSA_HASH160');
    });

    it('should allow to set read only', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      publicKeyInCreation.readOnly = true;

      expect(publicKeyInCreation.readOnly).to.equal(true);
    });

    it('should allow to set data', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      publicKeyInCreation.data = Buffer.from('333333333334001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex');

      expect(Buffer.from(publicKeyInCreation.data)).to.deep.equal(Buffer.from('333333333334001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'));
    });

    it('should allow to set signature', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      publicKeyInCreation.signature = [1, 2, 3, 4, 5, 6];

      expect([...publicKeyInCreation.signature]).to.deep.equal([1, 2, 3, 4, 5, 6]);
    });
  });

  describe('conversion methods', () => {
    it('should round-trip via toJSON/fromJSON', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      const json = publicKeyInCreation.toJSON();
      expect(json).to.be.an('object');
      expect(json.id).to.equal(0);
      expect(json.purpose).to.equal(0); // AUTHENTICATION = 0
      expect(json.securityLevel).to.equal(0); // MASTER = 0
      expect(json.type).to.equal(0); // ECDSA_SECP256K1 = 0
      expect(json.readOnly).to.equal(false);

      const restored = wasm.IdentityPublicKeyInCreation.fromJSON(json);
      expect(restored.keyId).to.equal(publicKeyInCreation.keyId);
      expect(restored.purpose).to.equal(publicKeyInCreation.purpose);
      expect(restored.securityLevel).to.equal(publicKeyInCreation.securityLevel);
      expect(restored.keyType).to.equal(publicKeyInCreation.keyType);
      expect(restored.readOnly).to.equal(publicKeyInCreation.readOnly);
    });

    it('should export toObject', () => {
      const publicKeyInCreation = new wasm.IdentityPublicKeyInCreation(
        0,
        'AUTHENTICATION',
        'master',
        'ECDSA_SECP256K1',
        false,
        Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
        [],
      );

      const obj = publicKeyInCreation.toObject();
      // toObject exports with byte arrays which don't round-trip in serde_wasm_bindgen
      // but it should at least be defined
      expect(obj).to.not.be.undefined;
      expect(obj).to.be.an('object');
    });
  });
});
