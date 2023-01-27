const getChainAssetLockFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('ChainAssetLockProof', () => {
  let ChainAssetLockProof;
  let chainAssetLockProof;
  let chainAssetLockProofJS;

  before(async () => {
    ({ ChainAssetLockProof } = await loadWasmDpp());

    chainAssetLockProofJS = getChainAssetLockFixture();
    chainAssetLockProof = new ChainAssetLockProof(
      chainAssetLockProofJS.toObject(),
    );
  });

  describe('#getType', () => {
    it('should return correct type', () => {
      expect(chainAssetLockProof.getType())
        .to.equal(chainAssetLockProofJS.getType());
    });
  });

  describe('#getCoreChainLockedHeight', () => {
    it('should return correct coreChainLockedHeight', () => {
      expect(chainAssetLockProof.getCoreChainLockedHeight())
        .to.equal(chainAssetLockProofJS.getCoreChainLockedHeight());
    });
  });

  describe('#getOutPoint', () => {
    it('should return correct outPoint', () => {
      expect(chainAssetLockProof.getOutPoint())
        .to.deep.equal(chainAssetLockProofJS.getOutPoint());
    });
  });

  describe('#toJSON', () => {
    it('should return correct JSON', () => {
      expect(chainAssetLockProof.toJSON())
        .to.deep.equal(chainAssetLockProofJS.toJSON());
    });
  });

  describe('#toObject', () => {
    it('should return correct object', () => {
      expect(chainAssetLockProof.toObject())
        .to.deep.equal(chainAssetLockProofJS.toObject());
    });
  });

  describe('#createIdentifier', () => {
    it('should return correct identifier', () => {
      const identifier = chainAssetLockProof.createIdentifier();
      const identifierJS = chainAssetLockProofJS.createIdentifier();

      expect(identifier.toBuffer())
        .to.deep.equal(identifierJS.toBuffer());
    });
  });
});
