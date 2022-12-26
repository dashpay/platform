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

  describe('#createIdentifier', () => {
    it('should return correct identifier', () => {
      const identifier = instantAssetLockProof.createIdentifier();
      const identifierJS = instantAssetLockProofJS.createIdentifier();

      expect(identifier.toBuffer())
        .to.deep.equal(identifierJS.toBuffer());
    });
  });

  describe('#getInstantLock', () => {
    it('should return correct instant lock', () => {
      const instantLock = instantAssetLockProof.getInstantLock();
      const instantLockJS = instantAssetLockProofJS.getInstantLock();

      expect(instantLock)
        .to.deep.equal(instantLockJS.toBuffer());
    });
  });

  describe('#getTransaction', () => {
    it('should return correct transaction', () => {
      const transaction = instantAssetLockProof.getTransaction();
      const transactionJS = instantAssetLockProofJS.getTransaction();

      expect(transaction)
        .to.deep.equal(transactionJS.toBuffer());
    });
  });

  describe('#toObject', () => {
    it('should return correct object', () => {
      expect(instantAssetLockProof.toObject())
        .to.deep.equal(instantAssetLockProofJS.toObject());
    });
  });

  describe('#toJSON', () => {
    it('should return correct JSON', () => {
      expect(instantAssetLockProof.toJSON())
        .to.deep.equal(instantAssetLockProofJS.toJSON());
    });
  });
});
