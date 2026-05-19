import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { fromHexString } from './utils/hex.ts';

before(async () => {
  await initWasm();
});

interface PublicKeyInCreationOptions {
  keyId?: number;
  purpose?: string;
  securityLevel?: string;
  keyType?: string;
  isReadOnly?: boolean;
  data?: Uint8Array;
  signature?: number[];
}

interface UpdateTransitionOptions {
  identityId?: string;
  revision?: bigint;
  nonce?: bigint;
  addPublicKeys?: unknown[];
  disablePublicKeys?: number[];
  userFeeIncrease?: number;
}

describe('IdentityUpdateTransition', () => {
  // Helper to create a public key in creation
  function createPublicKeyInCreation(options: PublicKeyInCreationOptions = {}) {
    return new wasm.IdentityPublicKeyInCreation({
      keyId: options.keyId ?? 1,
      purpose: options.purpose ?? 'SYSTEM',
      securityLevel: options.securityLevel ?? 'master',
      keyType: options.keyType ?? 'ECDSA_SECP256K1',
      isReadOnly: options.isReadOnly ?? false,
      data: options.data ?? fromHexString('036a394312e40e81d928fde2bde7880070e4fa9c1d1d9b168da707ea468afa2b48'),
      signature: options.signature ?? [],
    });
  }

  // Helper to create an update transition
  function createUpdateTransition(options: UpdateTransitionOptions = {}) {
    return new wasm.IdentityUpdateTransition({
      identityId: options.identityId ?? 'GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3',
      revision: options.revision ?? BigInt(1),
      nonce: options.nonce ?? BigInt(1),
      addPublicKeys: options.addPublicKeys ?? [],
      disablePublicKeys: options.disablePublicKeys ?? [],
      userFeeIncrease: options.userFeeIncrease ?? 0,
    });
  }

  describe('constructor()', () => {
    it('should create IdentityUpdateTransition', () => {
      const transition = createUpdateTransition();

      expect(transition).to.be.an.instanceof(wasm.IdentityUpdateTransition);
    });

    it('should create IdentityUpdateTransition with key', () => {
      const key = createPublicKeyInCreation();

      const transition = createUpdateTransition({ addPublicKeys: [key] });

      expect(transition).to.be.an.instanceof(wasm.IdentityUpdateTransition);
      expect(key).to.be.an.instanceof(wasm.IdentityPublicKeyInCreation);
    });
  });

  describe('toBase64()', () => {
    it('should convert IdentityUpdateTransition to base64', () => {
      const transition = createUpdateTransition();

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('fromBase64()', () => {
    it('should restore IdentityUpdateTransition from base64', () => {
      const transition = createUpdateTransition();

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      const restored = wasm.IdentityUpdateTransition.fromBase64(base64);

      expect(restored.toBytes()).to.deep.equal(bytes);
    });
  });

  describe('revision', () => {
    it('should return revision', () => {
      const transition = createUpdateTransition();

      expect(transition.revision).to.deep.equal(BigInt(1));
    });

    it('should set revision', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.revision = BigInt(11111);

      expect(transition.revision).to.deep.equal(BigInt(11111));
    });
  });

  describe('nonce', () => {
    it('should return nonce', () => {
      const transition = createUpdateTransition();

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('should set nonce', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.nonce = BigInt(11111);

      expect(transition.nonce).to.deep.equal(BigInt(11111));
    });
  });

  describe('identityIdentifier', () => {
    it('should return identityIdentifier', () => {
      const transition = createUpdateTransition();

      expect(transition.identityIdentifier.toBase58()).to.deep.equal('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });

    it('should set identityIdentifier', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.identityIdentifier = '11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3';

      expect(transition.identityIdentifier.toBase58()).to.deep.equal('11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });
  });

  describe('publicKeyIdsToDisable', () => {
    it('should return publicKeyIdsToDisable', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      expect(Array.from(transition.publicKeyIdsToDisable)).to.deep.equal([11]);
    });

    it('should set publicKeyIdsToDisable', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.publicKeyIdsToDisable = [1, 2, 3, 4];

      expect(transition.publicKeyIdsToDisable).to.deep.equal(Uint32Array.from([1, 2, 3, 4]));
    });
  });

  describe('publicKeyIdsToAdd', () => {
    it('should return publicKeyIdsToAdd', () => {
      const key = createPublicKeyInCreation();

      const transition = createUpdateTransition({ addPublicKeys: [key], disablePublicKeys: [11] });

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(1);
    });

    it('should set publicKeyIdsToAdd', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      const key = createPublicKeyInCreation();

      transition.publicKeyIdsToAdd = [key, key];

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(2);
      expect(key).to.be.an.instanceof(wasm.IdentityPublicKeyInCreation);
    });
  });

  describe('userFeeIncrease', () => {
    it('should return userFeeIncrease', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11], userFeeIncrease: 1 });

      expect(transition.userFeeIncrease).to.deep.equal(1);
    });
  });

  describe('signature', () => {
    it('should return signature', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11], userFeeIncrease: 1 });

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should set signature', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.signature = [0, 1, 2, 3, 5];

      expect(transition.signature).to.deep.equal(Uint8Array.from([0, 1, 2, 3, 5]));
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should return signaturePublicKeyId', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });

    it('should set signaturePublicKeyId', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });
  });

  describe('toJSON()', () => {
    it('should produce expected JSON structure without keys', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      const json = transition.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.identityId).to.equal('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
      expect(json.revision).to.equal(1);
      expect(json.nonce).to.equal(1);
      expect(json.addPublicKeys).to.deep.equal([]);
      expect(json.disablePublicKeys).to.deep.equal([11]);
      expect(json.userFeeIncrease).to.equal(0);
      expect(json.signature).to.equal('');
      expect(json.signaturePublicKeyId).to.equal(0);
    });

    it('should produce expected JSON structure with keys', () => {
      const key = createPublicKeyInCreation();
      const transition = createUpdateTransition({ addPublicKeys: [key], disablePublicKeys: [11] });

      const json = transition.toJSON();

      expect(json.identityId).to.equal('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
      expect(json.addPublicKeys.length).to.equal(1);
      expect(json.addPublicKeys[0]).to.have.property('data');
      expect(json.disablePublicKeys).to.deep.equal([11]);
    });
  });

  describe('fromJSON()', () => {
    it('should restore transition from JSON and verify getters', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      const json = transition.toJSON();
      const restored = wasm.IdentityUpdateTransition.fromJSON(json);

      expect(restored.identityIdentifier.toBase58()).to.equal('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
      expect(restored.revision).to.deep.equal(BigInt(1));
      expect(restored.nonce).to.deep.equal(BigInt(1));
      expect(restored.publicKeyIdsToAdd.length).to.equal(0);
      expect(Array.from(restored.publicKeyIdsToDisable)).to.deep.equal([11]);
      expect(restored.userFeeIncrease).to.equal(0);
      expect(restored.signaturePublicKeyId).to.equal(0);
      expect(restored.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should restore transition with keys from JSON', () => {
      const key = createPublicKeyInCreation();
      const transition = createUpdateTransition({ addPublicKeys: [key], disablePublicKeys: [11] });

      const json = transition.toJSON();
      const restored = wasm.IdentityUpdateTransition.fromJSON(json);

      expect(restored.publicKeyIdsToAdd.length).to.equal(1);
      expect(Array.from(restored.publicKeyIdsToDisable)).to.deep.equal([11]);
    });
  });

  describe('toObject()', () => {
    it('should produce expected object structure', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      const obj = transition.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.identityId).to.be.instanceOf(Uint8Array);
      expect(obj.identityId.length).to.equal(32);
      expect(obj.revision).to.deep.equal(BigInt(1));
      expect(obj.nonce).to.deep.equal(BigInt(1));
      expect(obj.addPublicKeys).to.deep.equal([]);
      expect(obj.disablePublicKeys).to.deep.equal([11]);
      expect(obj.userFeeIncrease).to.equal(0);
      expect(obj.signature).to.be.instanceOf(Uint8Array);
      expect(obj.signaturePublicKeyId).to.equal(0);
    });
  });
});
