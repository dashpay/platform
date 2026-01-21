import getWasm from './helpers/wasm.js';
import { fromHexString } from './utils/hex.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('IdentityUpdateTransition', () => {
  // Helper to create a public key in creation
  function createPublicKeyInCreation(options = {}) {
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
  function createUpdateTransition(options = {}) {
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
    it('Should create IdentityUpdateTransition', () => {
      const transition = createUpdateTransition();

      expect(transition.__wbg_ptr).to.not.equal(0);
    });

    it('Should create IdentityUpdateTransition with key', () => {
      const key = createPublicKeyInCreation();

      const transition = createUpdateTransition({ addPublicKeys: [key] });

      expect(transition.__wbg_ptr).to.not.equal(0);
      expect(key.__wbg_ptr).to.not.equal(0);
    });

    it('Should convert IdentityUpdateTransition to base64 and back', () => {
      const transition = createUpdateTransition();

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.IdentityUpdateTransition.fromBase64(base64);

      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('getters', () => {
    it('Should return revision', () => {
      const transition = createUpdateTransition();

      expect(transition.revision).to.deep.equal(BigInt(1));
    });

    it('Should return nonce', () => {
      const transition = createUpdateTransition();

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('Should return identityIdentifier', () => {
      const transition = createUpdateTransition();

      expect(transition.identityIdentifier.toBase58()).to.deep.equal('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });

    it('Should return publicKeyIdsToDisable', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      expect(Array.from(transition.publicKeyIdsToDisable)).to.deep.equal([11]);
    });

    it('Should return publicKeyIdsToAdd', () => {
      const key = createPublicKeyInCreation();

      const transition = createUpdateTransition({ addPublicKeys: [key], disablePublicKeys: [11] });

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(1);
    });

    it('Should return userFeeIncrease', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11], userFeeIncrease: 1 });

      expect(transition.userFeeIncrease).to.deep.equal(1);
    });

    it('Should return signature', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11], userFeeIncrease: 1 });

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('Should return signature public key id', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });
  });

  describe('setters', () => {
    it('Should allow to set identityIdentifier', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.identityIdentifier = '11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3';

      expect(transition.identityIdentifier.toBase58()).to.deep.equal('11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });

    it('Should allow to set revision', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.revision = BigInt(11111);

      expect(transition.revision).to.deep.equal(BigInt(11111));
    });

    it('Should allow to set nonce', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.nonce = BigInt(11111);

      expect(transition.nonce).to.deep.equal(BigInt(11111));
    });

    it('Should allow to set publicKeyIdsToDisable', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.publicKeyIdsToDisable = [1, 2, 3, 4];

      expect(transition.publicKeyIdsToDisable).to.deep.equal(Uint32Array.from([1, 2, 3, 4]));
    });

    it('Should allow to set publicKeyIdsToAdd', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      const key = createPublicKeyInCreation();

      transition.publicKeyIdsToAdd = [key, key];

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(2);
      expect(key.__wbg_ptr).to.not.equal(0);
    });

    it('Should allow to set signature', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.signature = [0, 1, 2, 3, 5];

      expect(transition.signature).to.deep.equal(Uint8Array.from([0, 1, 2, 3, 5]));
    });

    it('Should allow to set signature public key id', () => {
      const transition = createUpdateTransition({ disablePublicKeys: [11] });

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });
  });
});
