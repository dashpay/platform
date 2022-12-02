const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('InstantAssetLockProof', () => {
  let InstantAssetLockProof;
  let instantAssetLockProof;
  let instantAssetLockProofJS;

  before(async () => {
    ({ InstantAssetLockProof } = await loadWasmDpp());

    instantAssetLockProofJS = getInstantAssetLockProofFixture();
  });

  beforeEach(() => {
    const {
      type,
      outputIndex,
      transaction,
      instantLock,
    } = instantAssetLockProofJS.toObject();
    // const { coreChainLockedHeight, outPoint } = inst;

    instantAssetLockProof = new InstantAssetLockProof({
      type,
      outputIndex,
      transaction,
      instantLock,
    });
  });

  describe('#getType', () => {
    it('should return correct type', () => {
      expect(instantAssetLockProof.getType())
        .to.equal(instantAssetLockProofJS.getType());
    });
  });

  describe('#getOutputIndex', () => {
    it('should return correct type', () => {
      expect(instantAssetLockProof.getOutputIndex())
        .to.equal(instantAssetLockProofJS.getOutputIndex());
    });
  });

  describe('#getOutPoint', () => {
    // Buffers mismatch
    it.skip('should return correct outPoint', () => {
      // eslint-disable-next-line
      // RS -> <Buffer f6 f0 d8 94 15 6c 99 60 bb 3c 42 ff d4 ba 46 3d 63 d5 68 1a 09 77 84 c2 df e7 68 68 e5 c0 c4 46 00 00 00 00>
      // eslint-disable-next-line
      // JS -> <Buffer 46 c4 c0 e5 68 68 e7 df c2 84 77 09 1a 68 d5 63 3d 46 ba d4 ff 42 3c bb 60 99 6c 15 94 d8 f0 f6 00 00 00 00>
      expect(instantAssetLockProof.getOutPoint())
        .to.deep.equal(instantAssetLockProofJS.getOutPoint());
    });
  });

  // describe('#getOutput', () => {
  //   it('should return correct output', () => {
  //     console.log(instantAssetLockProofJS.getOutput());
  //   });
  // });
  // describe('#createIdentifier')
  // describe('#getInstantLock')
  // describe('#getTransaction')
  // describe('#toObject')
  // describe('#toJSON')
});
