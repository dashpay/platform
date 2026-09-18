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
  totalBudget?: bigint;
  expiresAt?: bigint;
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
      totalBudget: options.totalBudget,
      expiresAt: options.expiresAt,
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

  describe('limits', () => {
    it('should register a key without limits as a version 0 key', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      expect(publicKeyInCreation.totalBudget).to.equal(undefined);
      expect(publicKeyInCreation.expiresAt).to.equal(undefined);
      expect(publicKeyInCreation.toObject().$formatVersion).to.equal('0');
    });

    it('should carry a budget and an expiry on a version 1 key', () => {
      const publicKeyInCreation = createPublicKeyInCreation({
        securityLevel: 'critical',
        totalBudget: BigInt(500000000),
        expiresAt: BigInt(1800000000000),
      });

      expect(publicKeyInCreation.totalBudget).to.equal(BigInt(500000000));
      expect(publicKeyInCreation.expiresAt).to.equal(BigInt(1800000000000));

      const obj = publicKeyInCreation.toObject();
      expect(obj.$formatVersion).to.equal('1');
      expect(obj.totalBudget).to.equal(BigInt(500000000));
      expect(obj.expiresAt).to.equal(BigInt(1800000000000));
      expect(obj.data).to.be.instanceOf(Uint8Array);
      expect(obj.data.length).to.equal(33);

      const json = publicKeyInCreation.toJSON();
      expect(json.$formatVersion).to.equal('1');
      expect(json.totalBudget).to.equal(500000000);
      expect(json.expiresAt).to.equal(1800000000000);
      expect(json.securityLevel).to.equal(1); // CRITICAL
    });

    it('should carry one limit and leave the other out', () => {
      const budgetOnly = createPublicKeyInCreation({ totalBudget: BigInt(10) });
      expect(budgetOnly.totalBudget).to.equal(BigInt(10));
      expect(budgetOnly.expiresAt).to.equal(undefined);
      expect(budgetOnly.toJSON()).to.not.have.property('expiresAt');

      const expiryOnly = createPublicKeyInCreation({ expiresAt: BigInt(30) });
      expect(expiryOnly.totalBudget).to.equal(undefined);
      expect(expiryOnly.expiresAt).to.equal(BigInt(30));
      expect(expiryOnly.toJSON()).to.not.have.property('totalBudget');
    });

    it('should round trip a limited key through fromObject() and fromJSON()', () => {
      const publicKeyInCreation = createPublicKeyInCreation({
        totalBudget: BigInt(500000000),
        expiresAt: BigInt(1800000000000),
      });

      const fromObject = wasm.IdentityPublicKeyInCreation.fromObject(publicKeyInCreation.toObject());
      expect(fromObject.totalBudget).to.equal(BigInt(500000000));
      expect(fromObject.expiresAt).to.equal(BigInt(1800000000000));
      expect(fromObject.keyId).to.equal(publicKeyInCreation.keyId);

      const fromJson = wasm.IdentityPublicKeyInCreation.fromJSON(publicKeyInCreation.toJSON());
      expect(fromJson.totalBudget).to.equal(BigInt(500000000));
      expect(fromJson.expiresAt).to.equal(BigInt(1800000000000));
      expect(Buffer.from(fromJson.data)).to.deep.equal(Buffer.from(publicKeyInCreation.data));
    });

    it('should turn a version 0 key into a version 1 key when a limit is set', () => {
      const publicKeyInCreation = createPublicKeyInCreation();

      publicKeyInCreation.totalBudget = BigInt(20);
      expect(publicKeyInCreation.totalBudget).to.equal(BigInt(20));
      expect(publicKeyInCreation.expiresAt).to.equal(undefined);
      expect(publicKeyInCreation.toObject().$formatVersion).to.equal('1');

      publicKeyInCreation.expiresAt = BigInt(40);
      expect(publicKeyInCreation.totalBudget).to.equal(BigInt(20));
      expect(publicKeyInCreation.expiresAt).to.equal(BigInt(40));
    });

    it('should carry the limits into the identity public key', () => {
      const publicKeyInCreation = createPublicKeyInCreation({
        totalBudget: BigInt(500000000),
        expiresAt: BigInt(1800000000000),
      });

      const publicKey = publicKeyInCreation.toIdentityPublicKey();

      expect(publicKey.totalBudget).to.equal(BigInt(500000000));
      expect(publicKey.expiresAt).to.equal(BigInt(1800000000000));
      expect(publicKey.toObject().$formatVersion).to.equal('1');
      expect(createPublicKeyInCreation().toIdentityPublicKey().totalBudget).to.equal(undefined);
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
