const { ChainAssetLockProof } = require('../../../../../dist');
const getChainAssetLockProofFixture = require('../../../../../lib/test/fixtures/getChainAssetLockProofFixture');

describe('ChainAssetLockProof', () => {
  let rawChainAssetLockProof;
  let chainAssetLockProof;

  before(async () => {
    rawChainAssetLockProof = getChainAssetLockProofFixture().toObject();
    chainAssetLockProof = new ChainAssetLockProof(
      rawChainAssetLockProof,
    );
  });

  describe('#getCoreChainLockedHeight', () => {
    it('should return correct coreChainLockedHeight', () => {
      expect(chainAssetLockProof.getCoreChainLockedHeight())
        .to.equal(rawChainAssetLockProof.coreChainLockedHeight);
    });
  });

  describe('#getOutPoint', () => {
    it('should return correct outPoint', () => {
      expect(chainAssetLockProof.getOutPoint())
        .to.deep.equal(rawChainAssetLockProof.outPoint);
    });
  });

  describe('#toObject', () => {
    it('should return correct object', () => {
      expect(chainAssetLockProof.toObject())
        .to.deep.equal(rawChainAssetLockProof);
    });
  });

  describe('#createIdentifier', () => {
    it('should return correct identifier', () => {
      const identifier = chainAssetLockProof.createIdentifier();

      expect(identifier.toBuffer())
        .to.have.length(32);
    });
  });
});
