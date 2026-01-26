import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { fromHexString } from './utils/hex.js';

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

  describe('serialization / deserialization', () => {
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

    it('should convert IdentityUpdateTransition to base64 and back', () => {
      const transition = createUpdateTransition();

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.IdentityUpdateTransition.fromBase64(base64);

      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('getters', () => {
    it('should return revision', () => {
      const transition = createUpdateTransition();

      expect(transition.revision).to.deep.equal(BigInt(1));
    });

    it('should return nonce', () => {
      const transition = createUpdateTransition();

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('should return identityIdentifier', () => {
      const transition = createUpdateTransition();

      expect(transition.identityIdentifier.toBase58()).to.deep.equal('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });

    it('should return publicKeyIdsToDisable', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      expect(Array.from(transition.publicKeyIdsToDisable)).to.deep.equal([11]);
    });

    it('should return publicKeyIdsToAdd', () => {
      const key = createPublicKeyInCreation();

      const transition = createUpdateTransition({ addPublicKeys: [key], disablePublicKeys: [11] });

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(1);
    });

    it('should return userFeeIncrease', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11], userFeeIncrease: 1 });

      expect(transition.userFeeIncrease).to.deep.equal(1);
    });

    it('should return signature', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11], userFeeIncrease: 1 });

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should return signature public key id', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });
  });

  describe('setters', () => {
    it('should allow to set identityIdentifier', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.identityIdentifier = '11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3';

      expect(transition.identityIdentifier.toBase58()).to.deep.equal('11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });

    it('should allow to set revision', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.revision = BigInt(11111);

      expect(transition.revision).to.deep.equal(BigInt(11111));
    });

    it('should allow to set nonce', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.nonce = BigInt(11111);

      expect(transition.nonce).to.deep.equal(BigInt(11111));
    });

    it('should allow to set publicKeyIdsToDisable', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.publicKeyIdsToDisable = [1, 2, 3, 4];

      expect(transition.publicKeyIdsToDisable).to.deep.equal(Uint32Array.from([1, 2, 3, 4]));
    });

    it('should allow to set publicKeyIdsToAdd', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      const key = createPublicKeyInCreation();

      transition.publicKeyIdsToAdd = [key, key];

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(2);
      expect(key).to.be.an.instanceof(wasm.IdentityPublicKeyInCreation);
    });

    it('should allow to set signature', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.signature = [0, 1, 2, 3, 5];

      expect(transition.signature).to.deep.equal(Uint8Array.from([0, 1, 2, 3, 5]));
    });

    it('should allow to set signature public key id', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });
  });
});
