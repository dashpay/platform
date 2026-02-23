import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

interface PublicKeyInCreationOptions {
  keyId?: number;
  purpose?: string;
  securityLevel?: string;
  keyType?: string;
  isReadOnly?: boolean;
  data?: Uint8Array | Buffer;
  signature?: number[];
}

describe('IdentityPublicKeyInCreation', () => {
  // Helper function to create a public key in creation with options object
  function createPublicKeyInCreation(options: PublicKeyInCreationOptions = {}) {
    return new wasm.IdentityPublicKeyInCreation({
      keyId: options.keyId ?? 0,
      purpose: options.purpose ?? 'AUTHENTICATION',
      securityLevel: options.securityLevel ?? 'master',
      keyType: options.keyType ?? 'ECDSA_SECP256K1',
      isReadOnly: options.isReadOnly ?? false,
      data: options.data ?? Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'),
      signature: options.signature ?? [],
    });
  }

  describe('constructor()', () => {
    it('should create from values', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(publicKeyInCreation).to.be.an.instanceof(wasm.IdentityPublicKeyInCreation);
    });
  });

  describe('toIdentityPublicKey()', () => {
    it('should convert to IdentityPublicKey', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      const publicKey = publicKeyInCreation.toIdentityPublicKey();

      expect(publicKeyInCreation).to.be.an.instanceof(wasm.IdentityPublicKeyInCreation);
      expect(publicKey.constructor.name).to.equal('IdentityPublicKey');
    });
  });

  describe('keyId', () => {
    it('should get keyId', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(publicKeyInCreation.keyId).to.equal(0);
    });

    it('should set keyId', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.keyId = 123;

      expect(publicKeyInCreation.keyId).to.equal(123);
    });
  });

  describe('purpose', () => {
    it('should get purpose', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(publicKeyInCreation.purpose).to.equal('AUTHENTICATION');
    });

    it('should set purpose', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.purpose = 'OWNER';

      expect(publicKeyInCreation.purpose).to.equal('OWNER');
    });
  });

  describe('securityLevel', () => {
    it('should get securityLevel', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(publicKeyInCreation.securityLevel).to.equal('MASTER');
    });

    it('should set securityLevel', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.securityLevel = 'critical';

      expect(publicKeyInCreation.securityLevel).to.equal('CRITICAL');
    });
  });

  describe('keyType', () => {
    it('should get keyType', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(publicKeyInCreation.keyType).to.equal('ECDSA_SECP256K1');
    });

    it('should set keyType', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.keyType = 'ECDSA_HASH160';

      expect(publicKeyInCreation.keyType).to.equal('ECDSA_HASH160');
    });
  });

  describe('isReadOnly', () => {
    it('should get isReadOnly', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(publicKeyInCreation.isReadOnly).to.equal(false);
    });

    it('should set isReadOnly', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.isReadOnly = true;

      expect(publicKeyInCreation.isReadOnly).to.equal(true);
    });
  });

  describe('data', () => {
    it('should get data', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(Buffer.from(publicKeyInCreation.data)).to.deep.equal(Buffer.from('0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'));
    });

    it('should set data', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.data = Buffer.from('333333333334001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex');

      expect(Buffer.from(publicKeyInCreation.data)).to.deep.equal(Buffer.from('333333333334001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e', 'hex'));
    });
  });

  describe('signature', () => {
    it('should get signature', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect([...publicKeyInCreation.signature]).to.deep.equal([]);
    });

    it('should set signature', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.signature = [1, 2, 3, 4, 5, 6];

      expect([...publicKeyInCreation.signature]).to.deep.equal([1, 2, 3, 4, 5, 6]);
    });
  });

  describe('toJSON()', () => {
    it('should convert to JSON and back via fromJSON()', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

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
      expect(restored.isReadOnly).to.equal(publicKeyInCreation.isReadOnly);
    });
  });

  describe('toObject()', () => {
    it('should export to object', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      const obj = publicKeyInCreation.toObject();
      // toObject exports with byte arrays which don't round-trip in serde_wasm_bindgen
      // but it should at least be defined
      expect(obj).to.not.be.undefined();
      expect(obj).to.be.an('object');
    });
  });
});
