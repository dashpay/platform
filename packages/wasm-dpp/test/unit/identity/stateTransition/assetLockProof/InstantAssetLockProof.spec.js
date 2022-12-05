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
    it('should return correct outPoint', () => {
      expect(instantAssetLockProof.getOutPoint())
        .to.deep.equal(instantAssetLockProofJS.getOutPoint());
    });
  });

  describe('#getOutput', () => {
    it('should return correct output', () => {
      expect(instantAssetLockProof.getOutput())
        .to.deep.equal(instantAssetLockProofJS.getOutput().toObject());
    });
  });
  // describe('#createIdentifier')
  // describe('#getInstantLock')
  // describe('#getTransaction')
  // describe('#toObject')
  // describe('#toJSON')
});
