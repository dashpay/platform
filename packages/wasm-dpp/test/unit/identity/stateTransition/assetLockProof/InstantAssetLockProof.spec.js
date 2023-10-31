const getInstantAssetLockProofFixture = require('../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const { InstantAssetLockProof } = require('../../../../../dist');

describe('InstantAssetLockProof', () => {
  let instantAssetLockProof;
  let rawInstantAssetLockProof;

  before(async () => {
    rawInstantAssetLockProof = (await getInstantAssetLockProofFixture()).toObject();
    instantAssetLockProof = new InstantAssetLockProof(
      rawInstantAssetLockProof,
    );
  });

  describe('#getOutputIndex', () => {
    it('should return correct type', () => {
      expect(instantAssetLockProof.getOutputIndex())
        .to.equal(rawInstantAssetLockProof.outputIndex);
    });
  });

  describe('#getOutPoint', () => {
    it('should return correct outPoint', () => {
      expect(instantAssetLockProof.getOutPoint())
        .to.have.length(36);
    });
  });

  describe('#getOutput', () => {
    it('should return correct output', () => {
      expect(instantAssetLockProof.getOutput())
        .to.have.property('script');
      expect(instantAssetLockProof.getOutput())
        .to.have.property('satoshis');
    });
  });

  describe('#createIdentifier', () => {
    it('should return correct identifier', () => {
      const identifier = instantAssetLockProof.createIdentifier();

      expect(identifier.toBuffer())
        .to.have.length(32);
    });
  });

  describe('#getInstantLock', () => {
    it('should return correct instant lock', () => {
      const instantLock = instantAssetLockProof.getInstantLock();

      expect(instantLock)
        .to.deep.equal(rawInstantAssetLockProof.instantLock);
    });
  });

  describe('#getTransaction', () => {
    it('should return correct transaction', () => {
      const transaction = instantAssetLockProof.getTransaction();

      expect(transaction)
        .to.deep.equal(rawInstantAssetLockProof.transaction);
    });
  });

  describe('#toObject', () => {
    it('should return correct object', () => {
      expect(instantAssetLockProof.toObject())
        .to.deep.equal(rawInstantAssetLockProof);
    });
  });

  describe('#toJSON', () => {
    it('should return correct JSON', () => {
      expect(instantAssetLockProof.toJSON())
        .to.deep.equal({
          instantLock: instantAssetLockProof.getInstantLock().toString('base64'),
          outputIndex: instantAssetLockProof.getOutputIndex(),
          transaction: instantAssetLockProof.getTransaction().toString('hex'),
        });
    });
  });
});
