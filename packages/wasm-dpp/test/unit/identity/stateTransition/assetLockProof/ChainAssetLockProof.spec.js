const { hash } = require('../../../../../lib/utils/hash');
const getChainAssetLockFixture = require('../../../../../lib/test/fixtures/getChainAssetLockProofFixture');

const { ChainAssetLockProof } = require('../../../../../dist');

describe('ChainAssetLockProof', () => {
  let rawChainAssetLockProof;
  let chainAssetLockProof;

  before(async () => {
    rawChainAssetLockProof = getChainAssetLockFixture()
      .toObject();
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

  describe('#toJSON', () => {
    it('should return correct JSON', () => {
      expect(chainAssetLockProof.toJSON())
        .to.deep.equal({
          coreChainLockedHeight: rawChainAssetLockProof.coreChainLockedHeight,
          outPoint: rawChainAssetLockProof.outPoint.toString('base64'),
          type: rawChainAssetLockProof.type,
        });
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
        .to.deep.equal(hash(
          chainAssetLockProof.getOutPoint(),
        ));
    });
  });
});
