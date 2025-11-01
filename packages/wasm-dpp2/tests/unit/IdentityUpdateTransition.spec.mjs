import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('IdentityUpdateTransition', () => {
  describe('serialization / deserialization', () => {
    it('Should create IdentityUpdateTransition', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), 1, [], []);

      expect(transition.__wbg_ptr).to.not.equal(0);
    });

    it('Should create IdentityUpdateTransition with key', () => {
      const key = new wasm.IdentityPublicKeyInCreation(1, 'system', 'master', 'ECDSA_SECP256K1', false, [], []);

      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), 1, [key], []);

      expect(transition.__wbg_ptr).to.not.equal(0);
      expect(key.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('Should return revision', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), 1, [], []);

      expect(transition.revision).to.deep.equal(BigInt(1));
    });

    it('Should return nonce', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), 1, [], []);

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('Should return identityIdentifier', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), 1, [], []);

      expect(transition.identityIdentifier.base58()).to.deep.equal('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });

    it('Should return publicKeyIdsToDisable', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      expect(Array.from(transition.publicKeyIdsToDisable)).to.deep.equal([11]);
    });

    it('Should return publicKeyIdsToAdd', () => {
      const key = new wasm.IdentityPublicKeyInCreation(1, 'system', 'master', 'ECDSA_SECP256K1', false, [], []);

      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [key], [11]);

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(1);
    });

    it('Should return userFeeIncrease', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11], 1);

      expect(transition.userFeeIncrease).to.deep.equal(1);
    });

    it('Should return signature', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11], 1);

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('Should return signature public key id', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });
  });

  describe('setters', () => {
    it('Should allow to set identityIdentifier', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      transition.identityIdentifier = '11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3';

      expect(transition.identityIdentifier.base58()).to.deep.equal('11Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3');
    });

    it('Should allow to set revision', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      transition.revision = BigInt(11111);

      expect(transition.revision).to.deep.equal(BigInt(11111));
    });

    it('Should allow to set nonce', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      transition.nonce = BigInt(11111);

      expect(transition.nonce).to.deep.equal(BigInt(11111));
    });

    it('Should allow to set publicKeyIdsToDisable', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      transition.publicKeyIdsToDisable = [1, 2, 3, 4];

      expect(transition.publicKeyIdsToDisable).to.deep.equal(Uint32Array.from([1, 2, 3, 4]));
    });

    it('Should allow to set publicKeyIdsToAdd', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      const key = new wasm.IdentityPublicKeyInCreation(1, 'system', 'master', 'ECDSA_SECP256K1', false, [], []);

      transition.publicKeyIdsToAdd = [key, key];

      expect(transition.publicKeyIdsToAdd.length).to.deep.equal(2);
      expect(key.__wbg_ptr).to.not.equal(0);
    });

    it('Should allow to set signature', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      transition.signature = [0, 1, 2, 3, 5];

      expect(transition.signature).to.deep.equal(Uint8Array.from([0, 1, 2, 3, 5]));
    });

    it('Should allow to set signature public key id', () => {
      const transition = new wasm.IdentityUpdateTransition('GL2Rq8L3VuBEQfCAZykmUaiXXrsd1Bwub2gcaMmtNbn3', BigInt(1), BigInt(1), [], [11]);

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });
  });
});
