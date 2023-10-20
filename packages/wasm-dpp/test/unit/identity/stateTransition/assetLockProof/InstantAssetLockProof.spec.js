const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('InstantAssetLockProof', () => {
  let InstantAssetLockProof;
  let instantAssetLockProof;
  let Identifier;

  before(async () => {
    ({ InstantAssetLockProof, Identifier } = await loadWasmDpp());

    const instantAssetLockProofJS = getInstantAssetLockProofFixture();
    instantAssetLockProof = new InstantAssetLockProof(
      instantAssetLockProofJS.toObject(),
    );
  });

  describe('#getOutputIndex', () => {
    it('should return correct type', () => {
      expect(instantAssetLockProof.getOutputIndex())
        .to.equal(0);
    });
  });

  describe('#getOutPoint', () => {
    it('should return correct outPoint', () => {
      expect(Buffer.isBuffer(instantAssetLockProof.getOutPoint()))
        .to.be.true();
    });
  });

  describe('#getOutput', () => {
    it('should return correct output', () => {
      expect(Buffer.isBuffer(instantAssetLockProof.getOutput()))
        .to.be.true();
    });
  });

  describe('#createIdentifier', () => {
    it('should return correct identifier', () => {
      const identifier = instantAssetLockProof.createIdentifier();

      expect(identifier)
        .to.be.an.instanceOf(Identifier);
    });
  });

  describe('#getInstantLock', () => {
    it('should return correct instant lock', () => {
      const instantLock = instantAssetLockProof.getInstantLock();

      expect(Buffer.isBuffer(instantLock))
        .to.be.true();
    });
  });

  describe('#getTransaction', () => {
    it('should return correct transaction', () => {
      const transaction = instantAssetLockProof.getTransaction();

      expect(Buffer.isBuffer(transaction))
        .to.be.true();
    });
  });

  describe('#toObject', () => {
    it('should return correct object', () => {
      expect(instantAssetLockProof.toObject())
        .to.deep.equal({
          instantLock: instantAssetLockProof.getInstantLock(),
          outputIndex: instantAssetLockProof.getOutputIndex(),
          transaction: instantAssetLockProof.getTransaction(),
        });
    });
  });

  describe('#toJSON', () => {
    it('should return correct JSON', () => {
      expect(instantAssetLockProof.toJSON())
        .to.deep.equal({
          instantLock: instantAssetLockProof.getInstantLock().toString('base64'),
          outputIndex: instantAssetLockProof.getOutputIndex(),
          transaction: instantAssetLockProof.getTransaction().toString('base64'),
        });
    });
  });
});
