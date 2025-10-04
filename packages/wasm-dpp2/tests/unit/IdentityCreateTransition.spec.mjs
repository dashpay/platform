import getWasm from './helpers/wasm.js';
let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('IdentityCreateTransition', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create transition', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      expect(transition.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to serialize to bytes', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      const bytes = transition.bytes();

      expect(bytes.length > 0).to.equal(true);
    });

    it('should allow to deserialize to bytes', () => {
      const bytes = [0, 0, 0, 162, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 255, 255, 255, 255, 1, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

      const transition = wasm.IdentityCreateTransitionWASM.fromBytes(bytes);

      expect(transition.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get userFeeIncrease', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should allow to get AssetLock', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      expect(transition.assetLock.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to get Identifier', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      expect(transition.getIdentifier().base58()).to.equal('11111111111111111111111111111111');
    });

    it('should allow to get PublicKeys', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      expect(transition.publicKeys.length).to.equal(0);
    });

    it('should allow to get signature', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should allow to get signable bytes', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      expect(transition.getSignableBytes().length).to.equal(229);
    });
  });

  describe('setters', () => {
    it('should allow to set the userFeeIncrease', () => {
      const transition = wasm.IdentityCreateTransitionWASM.default(1);

      transition.userFeeIncrease = 100;

      expect(transition.userFeeIncrease).to.equal(100);
    });

    // TODO: Implement publickeys in creation setter
    // it('should allow to set the publicKeys', function () {
    //
    // })

    // TODO: Implement asset lock setter
    // it('should allow to set the asset lock', function () {
    //
    // })
  });
});
